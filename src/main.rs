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

use std::collections::HashMap;

use regex::Regex;
use hyper::{Body, Request, Response, Server};
use hyper::rt::Future;
use hyper::service::service_fn_ok;

#[derive(Debug, Clone)]
enum OpType {
    Plus,
    Minus,
    Div,
    Mul
}

impl OpType {
    pub fn compute(&self, first_number: f32, second_number: f32) -> Response<Body> {
        debug!("Data: {:?}, {:?}, {:?}", self, first_number, second_number);

        let response = OpResponse::create(
            match self {
                OpType::Plus => first_number + second_number,
                OpType::Minus => first_number - second_number,
                OpType::Div => first_number / second_number,
                OpType::Mul => first_number * second_number,
            }
        );

        Response::new(Body::from(response.into_json()))
    }
}

trait IntoJson: serde::Serialize {
    fn into_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    status: i16,
    error_message: String,
    error_code: u16
}

impl ErrorResponse {
    fn create(error_message: String, error_code: u16) -> Response<Body> {
        Response::builder()
            .status(error_code)
            .body(Body::from(ErrorResponse {
                status: -1,
                error_message: error_message.into(),
                error_code: error_code
            }.into_json()))
            .unwrap()
    }
}

impl IntoJson for ErrorResponse {}

#[derive(Serialize)]
struct OpResponse {
    status: i16,
    result: f32,
}

impl OpResponse {
    fn create(fpoint: f32) -> Self {
        OpResponse {
            status: 0,
            result: fpoint
        }
    }
}

impl IntoJson for OpResponse {}

lazy_static! {
    static ref METHODS_MAP: HashMap<&'static str, OpType> = {
        let mut map = HashMap::new();

        map.insert("plus", OpType::Plus);
        map.insert("minus", OpType::Minus);
        map.insert("div", OpType::Div);
        map.insert("mul", OpType::Mul);

        map
    };
}

fn check_method(method: &str) -> Option<OpType> {
    let clean_method = method.to_lowercase();

    METHODS_MAP.get(clean_method.as_str()).cloned()
}

fn check_number(number: &str) -> Result<f32, &str> {
    let result = serde_json::from_str(number).map_err(|_e| "Invalid number format. Should be a JSON number")?;

    if result < -1e9 {
        Err("Number should not be less than -1e9")
    } else if result > 1e9 {
        Err("Number should not be greater than 1e9")
    } else {
        Ok(result)
    }
}

fn router(req: Request<Body>) -> Response<Body> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^/([[:alpha:]]+)/(-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?)/(-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?)$").unwrap();
    }

    let caps = RE.captures(req.uri().path());

    match caps {
        Some(args) => {
            match check_method(args.get(1).unwrap().as_str()) {
                Some(method) => {
                    match (check_number(args.get(2).unwrap().as_str()),
                           check_number(args.get(3).unwrap().as_str())) {
                        (Ok(first_num), Ok(second_num)) => method.compute(first_num, second_num),
                        (Err(first_num_err), _) => ErrorResponse::create(format!("Invalid first argument: {}", first_num_err), 405),
                        (_, Err(second_num_err)) => ErrorResponse::create(format!("Invalid second argument: {}", second_num_err), 405),
                    }
                },
                _ => ErrorResponse::create(format!("Invalid method. Possible methods: {:?}", METHODS_MAP.keys()), 405)
            }
        },
        _ => {
            ErrorResponse::create("Invalid url".to_string(), 404)
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
