use tide::Request;
use tide::prelude::*;
use std::time::Instant;

async fn hello(req: Request<()>) -> tide::Result<String> {
    let res = "Hello world!".to_owned();
    Ok(res)
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    tide::log::start();
    let mut app = tide::new();
    app.at("/").get(hello);
    app.listen("127.0.0.1:8081").await?;
    Ok(())
}
