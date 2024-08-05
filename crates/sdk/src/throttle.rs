//! Provides a wrapper around behavior that throttles it when it is called concurrently or in quick
//! succession.
//!
//! Once an operation is completed, the next is delayed by the interval provided from the _start_
//! time of the previous operation.
//!
//! If multiple operations are delayed, they are pooled so that only one is run, and the rest clone
//! the resulting value.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{oneshot, Mutex};
use tokio::time::{self, Instant};

/// A type alias for a boxed future, to simplify places where it's used.
type BoxedFuture<'a, T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + Sync + 'a>>;

/// An operation that can be throttled.
///
/// We use this instead of `Fn() -> BoxedFuture<T>` because we run into strange and confusing
/// lifetime errors with that pattern.
pub trait Operation {
    type Output: Clone + Send + Sync;

    fn run(&self) -> impl std::future::Future<Output = Self::Output> + Send + Sync;
}

/// The state of the throttle at any given moment.
///
/// This is always wrapped in an `Arc<Mutex<...>>`.
enum ThrottleState<T> {
    /// On first run, the throttle is in this state. We never return to it.
    NeverRun,
    /// If the throttle was run before, but is not currently running, it is in this state.
    Idle { last_run: Instant },
    /// If the throttle is currently running, it is in this state. See [RunningState] for more
    /// details.
    Running { state: Arc<Mutex<RunningState<T>>> },
}

/// The state of a running operation.
///
/// This is independently wrapped in an `Arc<Mutex<...>>` because we would otherwise need to store
/// the result (in the `Finished` state) in the outer state, which means it would be possible (by
/// construction) for callers to acquire previous results instead of waiting for a new one. By
/// splitting this out, we ensure that once an operation has finished, a new one _must_ be started
/// in order to get a result.
enum RunningState<T> {
    Running(Vec<oneshot::Sender<T>>),
    Finished(T),
}

/// The throttle delays operations if they are called too quickly. It is constructed with some
/// behavior (implementing [Operation]) and an interval.
///
/// See the module-level documentation for details.
pub struct Throttle<Behavior: Operation> {
    behavior: Behavior,
    interval: Duration,
    state: Arc<Mutex<ThrottleState<Behavior::Output>>>,
}

impl<Behavior: Operation + Sync> Throttle<Behavior> {
    /// Constructs a new throttle with the given behavior and interval.
    pub fn new(behavior: Behavior, interval: Duration) -> Self {
        Self {
            behavior,
            interval,
            state: Arc::new(Mutex::new(ThrottleState::NeverRun)),
        }
    }

    /// Gets the next value, either by running the operation provided or by waiting for an
    /// already-running operation to complete.
    ///
    /// It may be delayed by up to the interval.
    pub async fn next(&self) -> Behavior::Output {
        self.decide().await.await
    }

    /// Decide what to do when asked to perform an operation.
    ///
    /// This acquires locks, but does not hold them in the returned future.
    async fn decide(&self) -> BoxedFuture<Behavior::Output> {
        let mut state = self.state.lock().await;

        // First, we check if we need to delay. If so, we store a future which will wait the
        // appropriate amount of time.
        //
        // We do not delay while holding onto the state lock.
        let delay: BoxedFuture<()> = match &*state {
            ThrottleState::NeverRun | ThrottleState::Running { .. } => Box::pin(async {}),
            ThrottleState::Idle { last_run } => {
                let delayed_start = *last_run + self.interval;
                Box::pin(time::sleep_until(delayed_start))
            }
        };

        match &*state {
            // If nothing is running, we start a new operation.
            ThrottleState::NeverRun | ThrottleState::Idle { last_run: _ } => {
                let start = Instant::now();
                let running_state = Arc::new(Mutex::new(RunningState::Running(Vec::new())));
                let running_state_for_later = running_state.clone();
                *state = ThrottleState::Running {
                    state: running_state,
                };
                drop(state);

                Box::pin(async move {
                    // First, we wait for the appropriate delay.
                    delay.await;

                    // Next, we run the operation to get the value.
                    let value = self.behavior.run().await;

                    // If any other operations are waiting, we let them know of the result.
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

                    // Finally, we mark the throttle state as idle.
                    let mut state = self.state.lock().await;
                    *state = ThrottleState::Idle { last_run: start };

                    value
                })
            }
            // If something is running, we wait for it by pushing a one-shot channel into a
            // queue, and then waiting for the result.
            ThrottleState::Running {
                state: running_state,
            } => {
                let mut running_state = running_state.lock().await;
                match &mut *running_state {
                    RunningState::Running(running) => {
                        let (sender, receiver) = oneshot::channel();
                        running.push(sender);
                        drop(running_state);
                        drop(state);

                        Box::pin(async { receiver.await.unwrap() })
                    }
                    // If it already finished since we acquired the outer lock, simply return
                    // the result.
                    RunningState::Finished(value) => {
                        let value = value.clone();
                        drop(running_state);
                        drop(state);

                        Box::pin(async move { value })
                    }
                }
            }
        }
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

    impl Operation for ThrottledCounter {
        type Output = i32;

        async fn run(&self) -> Self::Output {
            self.counter.fetch_and_inc()
        }
    }
}
