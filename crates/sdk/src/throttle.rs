use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{oneshot, Mutex};
use tokio::time::{self, Instant};

type BoxedFuture<'a, T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + Sync + 'a>>;

pub trait Throttled<T> {
    fn run(&self) -> impl std::future::Future<Output = T> + Send + Sync;
}

enum ThrottleState<T> {
    NeverRun,
    LastRun(Instant),
    Running {
        start: Instant,
        state: Arc<Mutex<RunningState<T>>>,
    },
}

enum RunningState<T> {
    Running(Vec<oneshot::Sender<T>>),
    Finished(T),
}

pub struct Throttle<T, Behavior: Throttled<T>> {
    behavior: Behavior,
    interval: Duration,
    state: Arc<Mutex<ThrottleState<T>>>,
    marker: PhantomData<T>,
}

impl<T: Clone + Send + Sync, Behavior: Throttled<T> + Sync> Throttle<T, Behavior> {
    pub fn new(behavior: Behavior, interval: Duration) -> Self {
        Self {
            behavior,
            interval,
            state: Arc::new(Mutex::new(ThrottleState::NeverRun)),
            marker: PhantomData,
        }
    }

    pub async fn next(&self) -> T {
        let action: BoxedFuture<T> = {
            let mut state = self.state.lock().await;
            let delay: BoxedFuture<()> = match &*state {
                ThrottleState::NeverRun => Box::pin(async {}),
                ThrottleState::LastRun(instant) => {
                    let delayed_start = *instant + self.interval;
                    Box::pin(time::sleep_until(delayed_start))
                }
                ThrottleState::Running { start, state: _ } => {
                    let delayed_start = *start + self.interval;
                    Box::pin(time::sleep_until(delayed_start))
                }
            };
            match &*state {
                ThrottleState::NeverRun | ThrottleState::LastRun(_) => {
                    let start = Instant::now();
                    let running_state = Arc::new(Mutex::new(RunningState::Running(Vec::new())));
                    let running_state_for_later = running_state.clone();
                    *state = ThrottleState::Running {
                        start,
                        state: running_state,
                    };
                    drop(state);

                    Box::pin(async move {
                        delay.await;

                        let value = self.behavior.run().await;

                        let mut state = self.state.lock().await;
                        *state = ThrottleState::LastRun(start);

                        let mut running_state = running_state_for_later.lock().await;
                        match &mut *running_state {
                            RunningState::Running(waiters) => {
                                for waiter in waiters.drain(..) {
                                    let _ = waiter.send(value.clone());
                                }
                                *running_state = RunningState::Finished(value.clone());
                            }
                            RunningState::Finished(_) => {
                                unreachable!("throttle completed twice");
                            }
                        }

                        value
                    })
                }
                ThrottleState::Running {
                    start: _,
                    state: running_state,
                } => {
                    let mut running_state = running_state.lock().await;
                    match &mut *running_state {
                        RunningState::Running(running) => {
                            let (sender, receiver) = oneshot::channel();
                            running.push(sender);
                            Box::pin(async { receiver.await.unwrap() })
                        }
                        RunningState::Finished(value) => {
                            let value = value.clone();
                            Box::pin(async move { value })
                        }
                    }
                }
            }
        };
        action.await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicI32, Ordering};

    use tokio::task;

    use super::*;

    #[tokio::test]
    async fn makes_requests() {
        time::pause();

        let counter = Counter::new();
        let throttled_counter = ThrottledCounter::new(counter.clone());
        let throttle = Arc::new(Throttle::new(throttled_counter, Duration::from_secs(1)));

        let handle = task::spawn({
            let t = throttle.clone();
            async move { t.next().await }
        });

        assert_eq!(counter.value(), 0);

        task::yield_now().await;
        assert_eq!(counter.value(), 1);

        assert_eq!(handle.await.unwrap(), 0);
    }

    #[tokio::test]
    async fn throttles_requests() {
        time::pause();

        let counter = Counter::new();
        let throttled_counter = ThrottledCounter::new(counter.clone());
        let throttle = Arc::new(Throttle::new(throttled_counter, Duration::from_secs(1)));

        let first = task::spawn({
            let t = throttle.clone();
            async move { t.next().await }
        });
        let second = task::spawn({
            let t = throttle.clone();
            async move { t.next().await }
        });

        assert_eq!(counter.value(), 0);

        task::yield_now().await;
        assert_eq!(counter.value(), 1);

        time::advance(Duration::from_millis(500)).await;
        task::yield_now().await;
        assert_eq!(counter.value(), 1);

        time::advance(Duration::from_millis(501)).await;
        task::yield_now().await;
        assert_eq!(counter.value(), 2);

        assert_eq!(first.await.unwrap(), 0);
        assert_eq!(second.await.unwrap(), 1);
    }

    #[tokio::test]
    async fn requests_are_pooled_during_throttling() {
        time::pause();

        let counter = Counter::new();
        let throttled_counter = ThrottledCounter::new(counter.clone());
        let throttle = Arc::new(Throttle::new(throttled_counter, Duration::from_secs(1)));

        let requests = [0, 1, 1, 1, 1].map(|expected_value| {
            (
                expected_value,
                task::spawn({
                    let t = throttle.clone();
                    async move { t.next().await }
                }),
            )
        });

        assert_eq!(counter.value(), 0);

        task::yield_now().await;
        assert_eq!(counter.value(), 1);

        time::advance(Duration::from_millis(500)).await;
        task::yield_now().await;
        assert_eq!(counter.value(), 1);

        time::advance(Duration::from_millis(501)).await;
        task::yield_now().await;
        assert_eq!(counter.value(), 2);

        time::advance(Duration::from_secs(60)).await;
        task::yield_now().await;
        assert_eq!(counter.value(), 2);

        for (value, request) in requests {
            assert_eq!(request.await.unwrap(), value);
        }
    }

    #[derive(Clone)]
    struct Counter(Arc<AtomicI32>);

    impl Counter {
        fn new() -> Self {
            Counter(Arc::new(AtomicI32::new(0)))
        }

        fn value(&self) -> i32 {
            self.0.load(Ordering::Acquire)
        }

        fn fetch_and_inc(&self) -> i32 {
            self.0.fetch_add(1, Ordering::AcqRel)
        }
    }

    struct ThrottledCounter {
        counter: Counter,
    }

    impl ThrottledCounter {
        fn new(counter: Counter) -> Self {
            Self { counter }
        }
    }

    impl Throttled<i32> for ThrottledCounter {
        async fn run(&self) -> i32 {
            self.counter.fetch_and_inc()
        }
    }
}
