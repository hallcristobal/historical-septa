use std::io::ErrorKind;

use super::async_trait;

use amqprs::{
    BasicProperties,
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{
        BasicAckArguments, BasicConsumeArguments, BasicPublishArguments, Channel,
        ExchangeDeclareArguments, QueueBindArguments, QueueDeclareArguments,
    },
    connection::{Connection, OpenConnectionArguments},
};
use serde::{Deserialize, Serialize};

pub struct RabbitQueue {
    connection: Connection,
    channel: Option<Channel>,
}

pub struct RabbitConnectionOpts<'a> {
    uri: &'a str,
    port: u16,
    user: &'a str,
    pass: &'a str,
}

impl<'a> RabbitConnectionOpts<'a> {
    pub fn new(uri: &'a str, port: u16, user: &'a str, pass: &'a str) -> Self {
        Self {
            uri,
            port,
            user,
            pass,
        }
    }
}

impl RabbitQueue {
    pub async fn new(opts: &RabbitConnectionOpts<'_>) -> anyhow::Result<Self> {
        let opts = OpenConnectionArguments::new(opts.uri, opts.port, opts.user, opts.pass);
        let connection = Connection::open(&opts).await.unwrap();
        connection
            .register_callback(DefaultConnectionCallback)
            .await
            .unwrap();
        Ok(RabbitQueue {
            connection,
            channel: None,
        })
    }

    pub async fn connect(&mut self) -> anyhow::Result<()> {
        let channel = self.connection.open_channel(None).await.unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();
        self.channel = Some(channel);
        Ok(())
    }

    pub async fn ensure_queue(&mut self, base_queue_name: &str) -> anyhow::Result<()> {
        if self.channel.is_none() {
            return Err(std::io::Error::from(ErrorKind::NotConnected).into());
        }

        let queue_name = format!("q.{}", base_queue_name);
        let exchange_name = format!("x.{}", base_queue_name);

        let channel = self.channel.as_ref().unwrap();
        let (queue_name, _, _) = channel
            .queue_declare(
                QueueDeclareArguments::new(&queue_name)
                    .auto_delete(false)
                    .finish(),
            )
            .await
            .unwrap()
            .unwrap();
        channel
            .exchange_declare(
                ExchangeDeclareArguments::new(&exchange_name, "fanout")
                    .durable(false)
                    .finish(),
            )
            .await
            .unwrap();
        channel
            .queue_bind(QueueBindArguments::new(&queue_name, &exchange_name, ""))
            .await
            .unwrap();

        Ok(())
    }

    pub async fn ensure_exchange(&mut self, base_exchange_name: &str) -> anyhow::Result<()> {
        if self.channel.is_none() {
            return Err(std::io::Error::from(ErrorKind::NotConnected).into());
        }
        let exchange_name = format!("x.{}", base_exchange_name);

        let channel = self.channel.as_ref().unwrap();
        channel
            .register_callback(DefaultChannelCallback)
            .await
            .unwrap();
        channel
            .exchange_declare(
                ExchangeDeclareArguments::new(&exchange_name, "fanout")
                    .durable(false)
                    .auto_delete(false)
                    .finish(),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub async fn consume<ConsumeFn, MessgeContent>(
        &mut self,
        queue_name: &str,
        mut consume: ConsumeFn,
    ) -> anyhow::Result<()>
    where
        ConsumeFn: AsyncFnMut(MessgeContent) -> std::io::Result<()>,
        MessgeContent: for<'a> Deserialize<'a>,
    {
        if self.channel.is_none() {
            return Err(std::io::Error::from(ErrorKind::NotConnected).into());
        }
        let channel = self.channel.as_ref().unwrap();

        let queue_name = format!("q.{}", queue_name);
        let args = BasicConsumeArguments::new(&queue_name, "")
            .manual_ack(true)
            .finish();

        let (_, mut message_recv) = channel.basic_consume_rx(args).await.unwrap();
        while let Some(message) = message_recv.recv().await {
            if let Some(content) = message.content {
                let content = serde_json::from_slice::<MessgeContent>(content.as_slice()).unwrap();
                match consume(content).await {
                    Ok(_) => {
                        channel
                            .basic_ack(BasicAckArguments::new(
                                message.deliver.unwrap().delivery_tag(),
                                false,
                            ))
                            .await
                            .unwrap();
                    }
                    Err(e) => {
                        error!("Error consuming file: {:?}", e);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn send<MessageContent>(
        &mut self,
        exchange_name: &str,
        content: MessageContent,
    ) -> anyhow::Result<()>
    where
        MessageContent: Serialize,
    {
        if self.channel.is_none() {
            return Err(std::io::Error::from(ErrorKind::NotConnected).into());
        }
        let channel = self.channel.as_ref().unwrap();

        let args = BasicPublishArguments::new(&format!("x.{}", exchange_name), "-");
        let props = BasicProperties::default();
        let content = serde_json::to_vec(&content).unwrap();
        channel.basic_publish(props, content, args).await.unwrap();

        Ok(())
    }
}

#[async_trait]
impl super::GenericQueue for RabbitQueue {
    type Opts<'init> = RabbitConnectionOpts<'init>;

    async fn init<'init>(opts: Self::Opts<'init>) -> anyhow::Result<Self> {
        self::RabbitQueue::new(&opts).await
    }
}
