use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::future::FutureExt;
use futures::stream::Stream;

pub enum FutureStream<'a, T> {
    Future(Pin<Box<dyn Future<Output = Self> + 'a>>),
    Stream(Pin<Box<dyn Stream<Item = T> + 'a>>),
}

impl<'a, T> FutureStream<'a, T> {
    pub fn future<F, S>(future: F) -> Self
    where
        F: Future<Output = S> + 'a,
        S: Stream<Item = T> + 'a,
        T: 'a,
    {
        Self::Future(Box::pin(future.map(Self::stream)))
    }

    pub fn stream<S>(stream: S) -> Self
    where
        S: Stream<Item = T> + 'a,
    {
        Self::Stream(Box::pin(stream))
    }
}

impl<'a, T> Stream for FutureStream<'a, T>
where
    T: 'a,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match *self.as_mut() {
            Self::Stream(ref mut stream) => stream.as_mut().poll_next(cx),
            Self::Future(ref mut future) => {
                if let Poll::Ready(stream) = future.as_mut().poll(cx) {
                    self.set(stream);
                    cx.waker().wake_by_ref();
                }

                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures::stream::{iter, StreamExt};
    use futures_timer::Delay;

    use super::FutureStream;

    #[tokio::test]
    async fn test_future() {
        let mut stream = FutureStream::future(async {
            Delay::new(Duration::from_millis(100)).await;

            iter(vec!["a", "b", "c"])
        });

        assert_eq!(stream.next().await, Some("a"));
        assert_eq!(stream.next().await, Some("b"));
        assert_eq!(stream.next().await, Some("c"));
        assert_eq!(stream.next().await, None);
    }

    #[tokio::test]
    async fn test_stream() {
        let mut stream = FutureStream::stream(iter(vec!["d", "e", "f"]));

        assert_eq!(stream.next().await, Some("d"));
        assert_eq!(stream.next().await, Some("e"));
        assert_eq!(stream.next().await, Some("f"));
        assert_eq!(stream.next().await, None);
    }
}
