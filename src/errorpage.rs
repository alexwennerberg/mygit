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
impl<State: Clone + Send + Sync + 'static> Middleware<State> for ErrorToErrorpage {
    async fn handle(&self, req: Request<State>, next: Next<'_, State>) -> tide::Result {
        let resource = req.url().path().to_string();
        let method = req.method();
        let mut response = next.run(req).await;
        if let Some(err) = response.take_error() {
            let status = err.status();

            if method == tide::http::Method::Head {
                // the server MUST NOT send a message body in the response
                // - RFC 7231 § 4.3.2
                response.take_body();
            } else {
                let message = match status {
                    // don't expose 500 error
                    StatusCode::InternalServerError if !cfg!(debug_assertions) =>  "Internal Server Error".to_owned(),
                    _ => err.into_inner().to_string(),
                };
                response = ErrorTemplate {
                    resource,
                    status,
                    message: message
                }
                .into();
                response.set_status(status);
            }

            if status == 405 {
                // The origin server MUST generate an Allow header field in
                // a 405 response containing a list of the target
                // resource's currently supported methods.
                // - RFC 7231 § 6.5.5
                //
                // We only ever support GET or HEAD requests.
                // tide adds support for HEAD automatically if we implement GET
                response.insert_header("Allow", "GET, HEAD");
            }
        }

        response.insert_header("Permissions-Policy", "interest-cohort=()");
        Ok(response)
    }
}
