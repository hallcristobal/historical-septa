use async_trait::async_trait;

mod rabbit;

#[async_trait]
pub trait GenericQueue
where
    Self: Sized,
{
    type Opts<'init>;
    async fn init<'init>(opts: Self::Opts<'init>) -> anyhow::Result<Self>;
}

pub mod prelude {
    pub use super::GenericQueue;
    pub use super::rabbit::RabbitQueue as Queue;
    pub use super::rabbit::RabbitConnectionOpts as Opts;
}
