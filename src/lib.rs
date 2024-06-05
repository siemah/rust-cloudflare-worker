use serde::{Deserialize, Serialize};
use worker::*;
use reqwest;

const API_URL: &str = "https://soukesmar.com/wp-json";

#[derive(Debug, Deserialize, Serialize)]
struct GenericResponse {
    status: u16,
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Image {
    id: i16,
    date_created: String,
    date_created_gmt: String,
    date_modified: String,
    date_modified_gmt: String,
    src: String,
    name: String,
    alt: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Product {
    id: i16,
    slug: String,
    name: String,
    description: String,
    status: String,
    price: String,
    regular_price: String,
    sale_price: String,
    images: Vec<Image>,
}

#[event(fetch)]
async fn main(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let cache_duration = 600;
    let original_url = req.url()?;
    let path_seach = format!("{}?{}", original_url.path(), original_url.query().unwrap_or(""));
    let endpoint = format!("{}{}", API_URL, path_seach);
    let req_instance = Request::new(endpoint.as_str(), req.method())?;
    let cache = Cache::default();
    let response = cache.get(&req_instance, false).await?;
    //
    match response {
        None => {
            let client = reqwest::Client::new();
            let response = client.get(&endpoint)
                .send()
                .await
                .map_err(|e| {
                    console_log!("Failed to fetch data from the API: {:?}", e.to_string());
                    return format!("Failed to fetch data: {}", e);
                })?;
            let fetch_response_headers = response.headers();

            if !response.status().is_success() {
                console_log!("fetching failed");
                return Err("failed to fetch data".into());
            } else {
                console_log!("fetching success");
            }

            let mut headers = Headers::new();
            fetch_response_headers.iter().for_each(|(key, value)| {
                headers.set(key.as_str(), value.to_str().unwrap()).unwrap();
            });
            headers.delete("link")?;
            headers.delete("expires")?;
            let results: Vec<Product> = response.json().await.unwrap();
            let mut new_response = Response::from_json(&results).unwrap();
            headers.set("Cache-Control", format!("s-max-age={}", &cache_duration).as_str())?;
            headers.into_iter().for_each(|(key, value)| {
                if key.to_lowercase() != "date" || key.to_lowercase() != "link" {
                    new_response
                        .headers_mut()
                        .set(key.as_str(), value.as_str())
                        .unwrap();
                }
            });
            new_response
                .headers_mut()
                .set("Cache-Control", format!("s-max-age={}", &cache_duration).as_str())?;
            let cloned_response = new_response.cloned();
            Cache::default()
                .put(&req_instance, Response::from(new_response))
                .await?;
            // Response::from_json(&results)
            cloned_response
        }
        Some(cached_response) => {
            Ok(cached_response)
        }
    }
}

// pub async fn handle_get(_: Request, _ctx: RouteContext<()>) -> worker::Result<Response> {
//     Response::from_json(&GenericResponse {
//         status: 200,
//         message: "You reached a GET route!".to_string(),
//     })
// }