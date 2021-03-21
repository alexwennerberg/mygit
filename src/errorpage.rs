use askama::Template;
use tide::{Middleware, Next, Request, StatusCode};

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    resource: String,
    status: StatusCode,
    message: String,
}

pub struct ErrorToErrorpage;

#[async_trait::async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for ErrorToErrorpage{
    async fn handle(&self, req: Request<State>, next: Next<'_, State>) -> tide::Result {
        let resource = req.url().path().to_string();
        let mut response = next.run(req).await;
        if let Some(err) = response.take_error() {
            let status = err.status();
            response = ErrorTemplate{
                resource,
                status,
                message: err.into_inner().to_string(),
            }.into();
            response.set_status(status);
        }

        Ok(response)
    }
}
