use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::{self, Instant};

pub trait Throttled<T> {
    fn run(&self) -> impl std::future::Future<Output = T> + Send;
}

pub struct Throttle<T, Behavior: Throttled<T>> {
    behavior: Behavior,
    interval: Duration,
    last_run: Arc<Mutex<Option<Instant>>>,
    marker: PhantomData<T>,
}

impl<T, Behavior: Throttled<T>> Throttle<T, Behavior> {
    pub fn new(behavior: Behavior, interval: Duration) -> Self {
        Self {
            behavior,
            interval,
            last_run: Arc::new(Mutex::new(None)),
            marker: PhantomData,
        }
    }

    pub async fn next(&self) -> T {
        {
            let mut last_run = self.last_run.lock().await;
            if let Some(last_run_value) = *last_run {
                time::sleep_until(last_run_value + self.interval).await;
            }
            *last_run = Some(Instant::now());
        }
        self.behavior.run().await
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
