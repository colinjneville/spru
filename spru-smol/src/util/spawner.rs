use std::{future::{Future, IntoFuture}, marker::PhantomData, rc::Rc, sync::Arc};

use smol::Task;

#[derive(Debug, Clone)]
pub struct LocalExecutorSpawner<'a>(Arc<smol::LocalExecutor<'a>>);

impl<'a> LocalExecutorSpawner<'a> {
    pub fn new(executor: &Arc<smol::LocalExecutor<'a>>) -> Self {
        Self(executor.clone())
    }

    pub fn spawn<T: 'a, F: Future<Output=T> + 'a>(&self, into_future: impl IntoFuture<Output=T, IntoFuture=F> + 'a) -> smol::Task<T> {
        let future = async move {
            let future = into_future.into_future();
            future.await
        };

        self.0.spawn(future)
    }

    pub fn remote_spawner(&self) -> RemoteExecutorSpawner<'a> {
        RemoteExecutorSpawner(self.0.clone())
    }
}

#[derive(Debug, Clone)]
pub struct RemoteExecutorSpawner<'a>(Arc<smol::LocalExecutor<'a>>);

unsafe impl<'a> Send for RemoteExecutorSpawner<'a> { }
unsafe impl<'a> Sync for RemoteExecutorSpawner<'a> { }

impl<'a> RemoteExecutorSpawner<'a> {
    pub fn new(executor: &Arc<smol::LocalExecutor<'a>>) -> Self {
        Self(executor.clone())
    }

    pub fn spawn<T: Send + 'a, F: Future<Output=T> + 'a>(&self, into_future: impl IntoFuture<Output=T, IntoFuture=F> + Send + Sync + 'a) -> smol::Task<T> {
        let future = async move {
            let future = into_future.into_future();
            future.await
        };

        self.0.spawn(future)
    }
}

// #[derive(Debug, Default)]
// struct InternalExecutor<'a>(smol::LocalExecutor<'a>);

// /// SAFETY: [smol::LocalExecutor] is an [smol::Executor] forced to be !Send to allow 
// /// loosened bounds on its impl functions. We remove this !Send restriction, and 
// /// un-loosen *most* bounds. (The implementation details of the executors are private, 
// /// so this is the only way to achieve our goals without duplicating the implementation 
// /// here). Our [LocalExecutor] once again re-adds !Send to mimic [smol::LocalExecutor].
// /// [RemoteExecutor] remains Send, but only allows spawning tasks onto the [LocalExecutor].
// /// In addition, these tasks take the form of an [IntoFuture] + Send implementation (but 
// /// the converted [Future] does *not* need to be Send). We need the remotely-created tasks
// /// to be Send to send to the [LocalExecutor], but once execution begins, they no longer need
// /// to be Send (as [LocalExecutor] itself is !Send, and will execute only on a single thread).
// /// Finally, the task output must be Send because the [RemoteExecutor]'s [smol::Task] may be
// /// on a different thread than the [LocalExecutor].
// unsafe impl<'a> Send for InternalExecutor<'a> { }
// unsafe impl<'a> Sync for InternalExecutor<'a> { }

// #[derive(Debug, Default, Clone)]
// pub struct LocalExecutor<'a> {
//     executor: Arc<InternalExecutor<'a>>,
//     // Re-make the executor non-Send and Sync
//     _non_send_sync: PhantomData<Rc<()>>,
// }

// impl<'a> LocalExecutor<'a> {
//     pub fn new() -> Self {
//         Self::default()
//     }

//     pub fn is_empty(&self) -> bool {
//         self.inner().is_empty()
//     }

//     pub fn remote_executor(&self) -> RemoteExecutor<'a> {
//         RemoteExecutor {
//             executor: self.executor.clone(),
//         }
//     }

//     pub fn spawn<T: 'a>(&self, future: impl Future<Output = T> + 'a) -> smol::Task<T> {
//         self.inner().spawn(future)
//     }

//     pub fn spawn_many<T: 'a, F: Future<Output = T> + 'a>(
//         &self,
//         futures: impl IntoIterator<Item = F>,
//         handles: &mut impl Extend<Task<F::Output>>,
//     ) {
//         self.inner().spawn_many(futures, handles)
//     }

//     pub fn try_tick(&self) -> bool {
//         self.inner().try_tick()
//     }

//     pub async fn tick(&self) {
//         self.inner().tick().await
//     }

//     pub async fn run<T>(&self, future: impl Future<Output = T>) -> T {
//         self.inner().run(future).await
//     }

//     fn inner(&self) -> &smol::LocalExecutor<'a> {
//         &self.executor.0
//     }
// }

// #[derive(Debug, Clone)]
// pub struct RemoteExecutor<'a> {
//     executor: Arc<InternalExecutor<'a>>,
// }

// impl<'a> RemoteExecutor<'a> {
//     pub fn spawn<T: Send + 'a, F: Future<Output=T> + 'a>(&self, into_future: impl IntoFuture<Output=T, IntoFuture=F> + Send + Sync + 'a) -> smol::Task<T> {
//         let future = async move {
//             let future = into_future.into_future();
//             future.await
//         };

//         self.executor.0.spawn(future)
//     }
// }

#[cfg(test)]
mod test {
    use futures_lite::FutureExt;

    use super::*;

    struct NonSendFuture(Rc<i32>);

    impl Future for NonSendFuture {
        type Output = i32;
    
        fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
            std::future::ready(*self.0).poll(cx)
        }
    }

    struct IntoNonSendFuture(i32);
    impl IntoFuture for IntoNonSendFuture {
        type Output = i32;
    
        type IntoFuture = NonSendFuture;
    
        fn into_future(self) -> Self::IntoFuture {
            NonSendFuture(Rc::new(self.0))
        }
    }

    #[test]
    fn remote_executor() {
        let local_ex = Arc::new(smol::LocalExecutor::new());

        let mut joins = vec![];

        for i in 1..=3 {
            let spawner = RemoteExecutorSpawner::new(&local_ex);
            joins.push(std::thread::spawn(move || {
                let into_non_send_future = IntoNonSendFuture(i);
                let task = spawner.spawn(into_non_send_future);
                let value = smol::future::block_on(task);
                value
            }));
        }

        let (send, recv) = onetime::channel();

        std::thread::spawn(move || {
            let mut result = [0, 0, 0];
            for (i, join) in joins.into_iter().enumerate() {
                result[i] = join.join().unwrap();
            }
            send.send(result).unwrap();
        });

        let result = smol::future::block_on(local_ex.run(async {
            recv.recv().await.unwrap()
        }));

        assert_eq!(result, [1, 2, 3]);
    }
}