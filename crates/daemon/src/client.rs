use anyhow::Result;
use proto::virtualos::virtual_os_client::VirtualOsClient;
use proto::virtualos::*;
use tonic::transport::{Channel, Endpoint};

pub struct Client {
    inner: VirtualOsClient<Channel>,
}

impl Client {
    pub async fn connect() -> Result<Option<Self>> {
        let socket = "/var/run/docklet.sock";
        // Check if socket exists
        if !std::path::Path::new(socket).exists() {
            return Ok(None);
        }
        let channel = Endpoint::try_from("unix:///var/run/docklet.sock")?
            .connect()
            .await?;
        let client = VirtualOsClient::new(channel);
        Ok(Some(Client { inner: client }))
    }

    pub async fn pull(&mut self, reference: &str, store_dir: &str) -> Result<()> {
        self.inner
            .pull(PullRequest {
                reference: reference.to_owned(),
                store_dir: store_dir.to_owned(),
            })
            .await?;
        Ok(())
    }

    pub async fn create(
        &mut self,
        id: Option<&str>,
        image: &str,
        command: &str,
        args: Vec<&str>,
        store_dir: &str,
        memory: Option<u64>,
        cpus: Option<f64>,
    ) -> Result<String> {
        let resp = self
            .inner
            .create(CreateRequest {
                id: id.unwrap_or("").to_owned(),
                image: image.to_owned(),
                command: command.to_owned(),
                args: args.iter().map(|s| s.to_string()).collect(),
                store_dir: store_dir.to_owned(),
                memory_limit: memory.unwrap_or(0),
                cpus: cpus.unwrap_or(0.0),
            })
            .await?;
        Ok(resp.into_inner().id)
    }

    pub async fn start(&mut self, id: &str) -> Result<()> {
        self.inner.start(StartRequest { id: id.to_owned() }).await?;
        Ok(())
    }

    pub async fn stop(&mut self, id: &str) -> Result<()> {
        self.inner.stop(StopRequest { id: id.to_owned() }).await?;
        Ok(())
    }

    pub async fn delete(&mut self, id: &str, force: bool) -> Result<()> {
        self.inner
            .delete(DeleteRequest {
                id: id.to_owned(),
                force,
            })
            .await?;
        Ok(())
    }

    pub async fn list(&mut self) -> Result<Vec<ContainerInfo>> {
        let resp = self.inner.list(ListRequest {}).await?;
        Ok(resp.into_inner().containers)
    }

    pub async fn run(
        &mut self,
        id: Option<&str>,
        image: &str,
        command: &str,
        args: Vec<&str>,
        store_dir: &str,
        memory: Option<u64>,
        cpus: Option<f64>,
        detach: bool,
        rm: bool,
    ) -> Result<String> {
        let resp = self
            .inner
            .run(RunRequest {
                id: id.unwrap_or("").to_owned(),
                image: image.to_owned(),
                command: command.to_owned(),
                args: args.iter().map(|s| s.to_string()).collect(),
                store_dir: store_dir.to_owned(),
                memory_limit: memory.unwrap_or(0),
                cpus: cpus.unwrap_or(0.0),
                detach,
                rm,
            })
            .await?;
        Ok(resp.into_inner().id)
    }
}
