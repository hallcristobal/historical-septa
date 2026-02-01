use async_trait::async_trait;
use serde::{Deserialize, Serialize};

mod rabbit;

#[async_trait]
pub trait GenericQueue
where
    Self: Sized,
{
    type Opts<'init>;
    async fn init<'init>(opts: Self::Opts<'init>) -> anyhow::Result<Self>;

    async fn consume<ConsumeFn, MessageContent, Fut>(
        &mut self,
        queue_name: &str,
        mut consume_fn: ConsumeFn
    ) -> anyhow::Result<()>
    where
        MessageContent: for<'a> Deserialize<'a>,
        ConsumeFn: Fn(MessageContent) -> Fut + Send,
        Fut: Future<Output = std::io::Result<()>> + Send;

    async fn send<MessageContent>(
        &mut self,
        exchange_name: &str,
        content: MessageContent,
    ) -> anyhow::Result<()>
    where
        MessageContent: Serialize + Send;
}

pub mod prelude {
    pub use super::GenericQueue;
    pub use super::rabbit::RabbitConnectionOpts as Opts;
    pub use super::rabbit::RabbitQueue as Queue;
}
