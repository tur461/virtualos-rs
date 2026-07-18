use crate::MyVirtualOs;
use engine::ResourceLimits;
use proto::virtualos::virtual_os_server::VirtualOs;
use proto::virtualos::*;
use storage::Store;
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl VirtualOs for MyVirtualOs {
    async fn pull(&self, request: Request<PullRequest>) -> Result<Response<PullResponse>, Status> {
        let req = request.into_inner();
        let store = Store::new(&req.store_dir);
        tokio::task::spawn_blocking(move || engine::pull_image(&req.reference, &store))
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(PullResponse {}))
    }

    async fn create(
        &self,
        request: Request<CreateRequest>,
    ) -> Result<Response<CreateResponse>, Status> {
        let req = request.into_inner();
        let store = Store::new(&req.store_dir);
        let manager = self.manager();
        let limits = ResourceLimits {
            memory: if req.memory_limit == 0 {
                None
            } else {
                Some(req.memory_limit)
            },
            cpus: if req.cpus == 0.0 {
                None
            } else {
                Some(req.cpus)
            },
        };
        let id_op = Some(req.id);
        let id = tokio::task::spawn_blocking(move || {
            manager.create(
                id_op.filter(|s: _| !s.is_empty()),
                &req.image,
                &req.command,
                req.args,
                &store,
                limits,
            )
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(CreateResponse { id: id.id }))
    }

    async fn start(
        &self,
        request: Request<StartRequest>,
    ) -> Result<Response<StartResponse>, Status> {
        let detached = true;
        let req = request.into_inner();
        let manager = self.manager();
        tokio::task::spawn_blocking(move || manager.start(&req.id, detached))
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(StartResponse {}))
    }

    async fn stop(&self, request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        let req = request.into_inner();
        let manager = self.manager();
        tokio::task::spawn_blocking(move || manager.stop(&req.id))
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(StopResponse {}))
    }

    async fn delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = request.into_inner();
        let manager = self.manager();
        tokio::task::spawn_blocking(move || {
            if req.force {
                // stop if running, then delete
                if manager.is_container_running(&req.id) {
                    manager.stop(&req.id)?;
                }
            }
            manager.delete(&req.id)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(DeleteResponse {}))
    }

    async fn list(&self, _request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        let manager = self.manager();
        let containers = tokio::task::spawn_blocking(move || manager.list())
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map_err(|e| Status::internal(e.to_string()))?;
        let mut response = ListResponse { containers: vec![] };
        for c in containers {
            response.containers.push(ContainerInfo {
                id: c.id,
                image: c.image,
                command: c.command,
                status: format!("{:?}", c.status),
                pid: c.pid.unwrap_or(0),
                network_ip: c.network_ip.unwrap_or_default(),
            });
        }
        Ok(Response::new(response))
    }

    async fn run(&self, request: Request<RunRequest>) -> Result<Response<RunResponse>, Status> {
        // For simplicity, run creates + starts, and if detach=false, we wait (but that would block the async thread).
        // In a real daemon, run with attach is tricky; we'll implement detached only for now and error on foreground.
        let req = request.into_inner();
        if !req.detach {
            return Err(Status::unimplemented(
                "Foreground run via daemon not supported yet",
            ));
        }
        let store = Store::new(&req.store_dir);
        let manager = self.manager();
        let limits = ResourceLimits {
            memory: if req.memory_limit == 0 {
                None
            } else {
                Some(req.memory_limit)
            },
            cpus: if req.cpus == 0.0 {
                None
            } else {
                Some(req.cpus)
            },
        };
        let id_op = Some(req.id);
        let id = tokio::task::spawn_blocking(move || {
            let container = manager.create(
                id_op.filter(|s| !s.is_empty()),
                &req.image,
                &req.command,
                req.args,
                &store,
                limits,
            )?;
            manager.start(&container.id, req.detach)?;
            Ok::<_, anyhow::Error>(container.id)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(RunResponse { id, pid: 0 })) // pid not known easily
    }
}
