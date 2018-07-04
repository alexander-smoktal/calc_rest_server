#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;
extern crate regex;

extern crate hyper;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use std::collections::HashSet;

use regex::Regex;
use hyper::{Body, Request, Response, Server};
use hyper::rt::Future;
use hyper::service::service_fn_ok;

trait IntoJson: serde::Serialize {
    fn into_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    status: i32,
    error_message: String,
    error_code: u32
}

impl IntoJson for ErrorResponse {}

#[derive(Serialize)]
struct OpResponse {
    status: i32,
    result: f32,
}

impl From<f32> for OpResponse {
    fn from(fpoint: f32) -> Self {
        OpResponse {
            status: 0i32,
            result: fpoint
        }
    }
}

impl IntoJson for OpResponse {}

fn calculate(method: String, first_number: f32, second_number: f32) -> Response<Body> {
    debug!("Data: {:?}, {:?}, {:?}", method, first_number, second_number);

    let response = OpResponse::from(
        match method.as_str() {
            "plus" => first_number + second_number,
            "minus" => first_number - second_number,
            "div" => first_number / second_number,
            "mul" => first_number * second_number,
            _ => unreachable!()
        }
    );
    
    Response::new(Body::from(response.into_json()))
}

fn error_response(error_message: &str, error_code: u32) -> Response<Body> {
    Response::builder()
        .status(error_code as u16)
        .body(Body::from(ErrorResponse { 
            status: -1i32,
            error_message: error_message.into(),
            error_code: error_code
        }.into_json()))
        .unwrap()
}

fn check_method(method: &str) -> Option<String> {
    let methods_set: HashSet<String> = ["plus", "minus", "div", "mul"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let clean_method = method.to_lowercase();

    if methods_set.contains(&clean_method) {
        Some(clean_method)
    } else {
        None
    }
}

fn check_number(number: &str) -> Option<f32> {
    number.parse::<f32>().ok()
}

fn router(req: Request<Body>) -> Response<Body> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^/([[:alpha:]]+)/([-+]?[0-9]*(\.[0-9]+([eE][-+]?[0-9]+)?)?)/([-+]?[0-9]*(\.[0-9]+([eE][-+]?[0-9]+)?)?)$").unwrap();
    }
    
    let caps = RE.captures(req.uri().path());
    
    match caps {
        Some(args) => {
            match check_method(args.get(1).unwrap().as_str()) {
                Some(method) => {
                    match (check_number(args.get(2).unwrap().as_str()),
                           check_number(args.get(5).unwrap().as_str())) {
                        (Some(first_num), Some(second_num)) => calculate(method, first_num, second_num),
                        _ => error_response("Invalid arguments", 404)
                    }
                },
                _ => error_response("Invalid method", 404)
            }
        },
        _ => {
            error_response("Malformed url", 404)
        }
    }
}

fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let new_svc = || {
        // service_fn_ok converts our function into a `Service`
        service_fn_ok(router)
    };

    let server = Server::bind(&addr)
        .serve(new_svc)
        .map_err(|e| eprintln!("server error: {}", e));

    hyper::rt::run(server);
}
