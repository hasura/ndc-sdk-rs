use std::marker::PhantomData;
use std::time::Duration;

#[async_trait::async_trait]
pub trait Throttled<T> {
    async fn run(&self) -> T;
}

pub struct Throttle<T, Behavior: Throttled<T>> {
    behavior: Behavior,
    _interval: Duration,
    marker: PhantomData<T>,
}

impl<T, Behavior: Throttled<T>> Throttle<T, Behavior> {
    pub fn new(behavior: Behavior, interval: Duration) -> Self {
        Self {
            behavior,
            _interval: interval,
            marker: PhantomData,
        }
    }

    pub async fn next(&self) -> T {
        self.behavior.run().await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    use tokio::{task, time};

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

        time::advance(Duration::from_millis(1)).await;
        assert_eq!(counter.value(), 1);

        assert_eq!(handle.await.unwrap(), 0);
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

    #[async_trait::async_trait]
    impl Throttled<i32> for ThrottledCounter {
        async fn run(&self) -> i32 {
            self.counter.fetch_and_inc()
        }
    }
}
