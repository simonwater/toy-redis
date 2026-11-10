use anyhow::Result;

use crate::{
    AofMiddleware, AuthMiddleware, CommandPipeline, ConnectionHandler, ConnectionState, Context,
    ReplConfMasterMiddleware, ReplicaMiddleware, Session, TransMiddleware, Value,
};
use std::sync::Arc;

pub fn start(ctx: Arc<Context>, conn: ConnectionHandler) -> Result<()> {
    let mut session = Session::new()
        .with_transaction()
        .with_subscription()
        .with_connection(conn);
    let cmd_pipeline = main_pipeline();
    // 读取命令
    while let Some(command) = session.connection_mut()?.receive_command()? {
        // 执行命令
        let response = match cmd_pipeline.execute(&command, &ctx, &mut session) {
            Ok(res) => res,
            Err(e) => {
                eprintln!("command pipeline execute error: {}", e);
                Value::SimpleErrors(format!("{}", e)).into()
            }
        };

        // 回写响应
        match response.run(&ctx, session)? {
            ConnectionState::KeepAlive(keep_session) => session = keep_session,
            ConnectionState::Close => break,
        }
    }
    Ok(())
}

fn main_pipeline() -> CommandPipeline {
    let pipeline = CommandPipeline::new();
    pipeline
        .use_middleware(Box::new(AuthMiddleware))
        .use_middleware(Box::new(TransMiddleware)) // 事务控制中间件
        .use_middleware(Box::new(ReplicaMiddleware)) // 复本同步中间件
        .use_middleware(Box::new(AofMiddleware)) // aof存储中间件
        .use_middleware(Box::new(ReplConfMasterMiddleware))
}
