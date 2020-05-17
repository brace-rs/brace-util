use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub enum FutureResult<'a, T, E> {
    Result(Box<Option<Result<T, E>>>),
    Future(Pin<Box<dyn Future<Output = Result<T, E>> + 'a>>),
}

impl<'a, T, E> FutureResult<'a, T, E> {
    pub fn from_ok(ok: T) -> Self {
        Self::Result(Box::new(Some(Ok(ok))))
    }

    pub fn from_err(err: E) -> Self {
        Self::Result(Box::new(Some(Err(err))))
    }

    pub fn from_result(result: Result<T, E>) -> Self {
        Self::Result(Box::new(Some(result)))
    }

    pub fn from_future<F>(future: F) -> Self
    where
        F: Future<Output = Result<T, E>> + 'a,
    {
        Self::Future(Box::pin(future))
    }
}

impl<'a, T, E> Future for FutureResult<'a, T, E> {
    type Output = Result<T, E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match *self.as_mut() {
            Self::Result(ref mut result) => Poll::Ready(result.take().expect("use after resolve")),
            Self::Future(ref mut future) => future.as_mut().poll(cx),
        }
    }
}

impl<'a, T, E> From<Result<T, E>> for FutureResult<'a, T, E> {
    fn from(result: Result<T, E>) -> Self {
        Self::from_result(result)
    }
}

#[cfg(test)]
mod tests {
    use super::FutureResult;

    #[derive(Debug, PartialEq)]
    struct Error;

    #[tokio::test]
    async fn test_future_result_from_ok() {
        let future = FutureResult::<String, Error>::from_ok(String::from("ok"));
        let result = future.await;

        assert_eq!(result, Ok(String::from("ok")));
    }

    #[tokio::test]
    async fn test_future_result_from_err() {
        let future = FutureResult::<String, Error>::from_err(Error);
        let result = future.await;

        assert_eq!(result, Err(Error));
    }

    #[tokio::test]
    async fn test_future_result_from_result() {
        let future = FutureResult::<&str, Error>::from(Ok("result"));
        let result = future.await;

        assert_eq!(result, Ok("result"));
    }

    #[tokio::test]
    async fn test_future_result_from_future() {
        let future = FutureResult::from_future(async {
            FutureResult::from_future(async {
                FutureResult::<&str, Error>::from(Ok("future")).await
            })
            .await
        });
        let result = future.await;

        assert_eq!(result, Ok("future"));
    }
}
