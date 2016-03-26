extern crate rustc_serialize;
#[macro_use] extern crate log;
#[macro_use] extern crate p_macro;
#[macro_use] extern crate libpact_v1_models;
#[macro_use] extern crate maplit;

use libpact_v1_models::model::{HttpPart, Request, Response, OptionalBody};
use std::collections::HashMap;
use std::any::Any;

mod json;

static BODY_MATCHERS: [(&'static str, fn(mismatches: &mut Vec<Mismatch>)); 1] = [
    ("application/json", json::match_json)
];

#[derive(Debug, Clone)]
pub enum Mismatch {
    MethodMismatch { expected: String, actual: String },
    PathMismatch { expected: String, actual: String },
    StatusMismatch { expected: u16, actual: u16 },
    QueryMismatch { parameter: String, expected: String, actual: String, mismatch: String },
    HeaderMismatch { key: String, expected: String, actual: String, mismatch: String },
    BodyTypeMismatch { expected: String, actual: String },
    BodyMismatch { path: String, expected: Option<String>, actual: Option<String>, mismatch: String }
}

impl PartialEq for Mismatch {
    fn eq(&self, other: &Mismatch) -> bool {
        match (self, other) {
            (&Mismatch::MethodMismatch{ expected: ref e1, actual: ref a1 },
                &Mismatch::MethodMismatch{ expected: ref e2, actual: ref a2 }) => {
                e1 == e2 && a1 == a2
            },
            (&Mismatch::PathMismatch{ expected: ref e1, actual: ref a1 },
                &Mismatch::PathMismatch{ expected: ref e2, actual: ref a2 }) => {
                e1 == e2 && a1 == a2
            },
            (&Mismatch::StatusMismatch{ expected: ref e1, actual: ref a1 },
                &Mismatch::StatusMismatch{ expected: ref e2, actual: ref a2 }) => {
                e1 == e2 && a1 == a2
            },
            (&Mismatch::BodyTypeMismatch{ expected: ref e1, actual: ref a1 },
                &Mismatch::BodyTypeMismatch{ expected: ref e2, actual: ref a2 }) => {
                e1 == e2 && a1 == a2
            },
            (&Mismatch::QueryMismatch{ parameter: ref p1, expected: ref e1, actual: ref a1, mismatch: ref m1 },
                &Mismatch::QueryMismatch{ parameter: ref p2, expected: ref e2, actual: ref a2, mismatch: ref m2 }) => {
                p1 == p2 && e1 == e2 && a1 == a2
            },
            (&Mismatch::HeaderMismatch{ key: ref p1, expected: ref e1, actual: ref a1, mismatch: ref m1 },
                &Mismatch::HeaderMismatch{ key: ref p2, expected: ref e2, actual: ref a2, mismatch: ref m2 }) => {
                p1 == p2 && e1 == e2 && a1 == a2
            },
            (&Mismatch::BodyMismatch{ path: ref p1, expected: ref e1, actual: ref a1, mismatch: ref m1 },
                &Mismatch::BodyMismatch{ path: ref p2, expected: ref e2, actual: ref a2, mismatch: ref m2 }) => {
                p1 == p2 && e1 == e2 && a1 == a2
            },
            (_, _) => false
        }
    }
}

pub enum DiffConfig {
    ALLOW_UNEXPECTED_KEYS,
    NO_UNEXPECTED_KEYS
}

pub fn match_text(mismatches: &mut Vec<Mismatch>) {

}

pub fn match_method(expected: String, actual: String, mismatches: &mut Vec<Mismatch>) {
    if expected.to_lowercase() != actual.to_lowercase() {
        mismatches.push(Mismatch::MethodMismatch { expected: expected, actual: actual });
    }
}

pub fn match_path(expected: String, actual: String, mismatches: &mut Vec<Mismatch>) {
    if expected != actual {
        mismatches.push(Mismatch::PathMismatch { expected: expected, actual: actual });
    }
}

fn compare_query_parameter_value(key: &String, expected: &String, actual: &String,
    mismatches: &mut Vec<Mismatch>) {
    if expected != actual {
        mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
            expected: expected.clone(),
            actual: actual.clone(),
            mismatch: format!("Expected '{}' but received '{}' for query parameter '{}'",
            expected, actual, key) });
    }
}

fn compare_query_parameter_values(key: &String, expected: &Vec<String>, actual: &Vec<String>,
    mismatches: &mut Vec<Mismatch>) {
    for (index, val) in expected.iter().enumerate() {
        if index < actual.len() {
            compare_query_parameter_value(key, val, &actual[index], mismatches);
        } else {
            mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: format!("{:?}", expected),
                actual: format!("{:?}", actual),
                mismatch: format!("Expected query parameter '{}' value '{}' but was missing", key, val) });
        }
    }
}

fn match_query_values(key: &String, expected: &Vec<String>, actual: &Vec<String>,
    mismatches: &mut Vec<Mismatch>) {
    if expected.is_empty() && !actual.is_empty() {
        mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
            expected: format!("{:?}", expected),
            actual: format!("{:?}", actual),
            mismatch: format!("Expected an empty parameter list for '{}' but received {:?}", key, actual) });
    } else {
        if expected.len() != actual.len() {
            mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: format!("{:?}", expected),
                actual: format!("{:?}", actual),
                mismatch: format!(
                    "Expected query parameter '{}' with {} value(s) but received {} value(s)",
                    key, expected.len(), actual.len()) });
        }
        compare_query_parameter_values(key, expected, actual, mismatches);
    }
}

fn match_query_maps(expected: HashMap<String, Vec<String>>, actual: HashMap<String, Vec<String>>,
    mismatches: &mut Vec<Mismatch>) {
    for (key, value) in &expected {
        match actual.get(key) {
            Some(actual_value) => match_query_values(key, value, actual_value, mismatches),
            None => mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: format!("{:?}", value),
                actual: "".to_string(),
                mismatch: format!("Expected query parameter '{}' but was missing", key) })
        }
    }
    for (key, value) in &actual {
        match expected.get(key) {
            Some(_) => (),
            None => mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: "".to_string(),
                actual: format!("{:?}", value),
                mismatch: format!("Unexpected query parameter '{}' received", key) })
        }
    }
}

pub fn match_query(expected: Option<HashMap<String, Vec<String>>>,
    actual: Option<HashMap<String, Vec<String>>>, mismatches: &mut Vec<Mismatch>) {
    match (actual, expected) {
        (Some(aqm), Some(eqm)) => match_query_maps(eqm, aqm, mismatches),
        (Some(aqm), None) => for (key, value) in &aqm {
            mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: "".to_string(),
                actual: format!("{:?}", value),
                mismatch: format!("Unexpected query parameter '{}' received", key) });
        },
        (None, Some(eqm)) => for (key, value) in &eqm {
            mismatches.push(Mismatch::QueryMismatch { parameter: key.clone(),
                expected: format!("{:?}", value),
                actual: "".to_string(),
                mismatch: format!("Expected query parameter '{}' but was missing", key) });
        },
        (None, None) => (),
    };
}

fn parse_charset_parameters(parameters: &[&str]) -> HashMap<String, String> {
    parameters.iter().map(|v| v.split("=").map(|p| p.trim()).collect::<Vec<&str>>())
        .fold(HashMap::new(), |mut map, name_value| {
            map.insert(name_value[0].to_string(), name_value[1].to_string());
            map
        })
}

fn match_content_type(expected: &String, actual: &String, mismatches: &mut Vec<Mismatch>) {
    let expected_values: Vec<&str> = libpact_v1_models::strip_whitespace(expected, ";");
    let actual_values: Vec<&str> = libpact_v1_models::strip_whitespace(actual, ";");
    let expected_parameters = expected_values.as_slice().split_first().unwrap();
    let actual_parameters = actual_values.as_slice().split_first().unwrap();
    let header_mismatch = Mismatch::HeaderMismatch { key: "Content-Type".to_string(),
        expected: format!("{}", expected),
        actual: format!("{}", actual),
        mismatch: format!("Expected header 'Content-Type' to have value '{}' but was '{}'",
            expected, actual) };

    if expected_parameters.0 == actual_parameters.0 {
        let expected_parameter_map = parse_charset_parameters(expected_parameters.1);
        let actual_parameter_map = parse_charset_parameters(actual_parameters.1);
        for (k, v) in expected_parameter_map {
            if actual_parameter_map.contains_key(&k) {
                if v != *actual_parameter_map.get(&k).unwrap() {
                    mismatches.push(header_mismatch.clone());
                }
            } else {
                mismatches.push(header_mismatch.clone());
            }
        }
    } else {
        mismatches.push(header_mismatch.clone());
    }
}

fn match_header_value(key: &String, expected: &String, actual: &String, mismatches: &mut Vec<Mismatch>) {
    if key.to_lowercase() == "content-type" {
        match_content_type(expected, actual, mismatches);
    } else if libpact_v1_models::strip_whitespace::<String>(expected, ",") != libpact_v1_models::strip_whitespace::<String>(actual, ",") {
        mismatches.push(Mismatch::HeaderMismatch { key: key.clone(),
            expected: format!("{}", expected),
            actual: format!("{}", actual),
            mismatch: format!("Expected header '{}' to have value '{}' but was '{}'", key, expected, actual) });
    }
}

fn find_entry(map: &HashMap<String, String>, key: &String) -> Option<(String, String)> {
    match map.keys().find(|k| k.to_lowercase() == key.to_lowercase() ) {
        Some(k) => map.get(k).map(|v| (key.clone(), v.clone()) ),
        None => None
    }
}

fn match_header_maps(expected: HashMap<String, String>, actual: HashMap<String, String>,
    mismatches: &mut Vec<Mismatch>) {
    for (key, value) in &expected {
        match find_entry(&actual, key) {
            Some((_, actual_value)) => match_header_value(key, value, &actual_value, mismatches),
            None => mismatches.push(Mismatch::HeaderMismatch { key: key.clone(),
                expected: format!("{:?}", value),
                actual: "".to_string(),
                mismatch: format!("Expected header '{}' but was missing", key) })
        }
    }
}

pub fn match_headers(expected: Option<HashMap<String, String>>,
    actual: Option<HashMap<String, String>>, mismatches: &mut Vec<Mismatch>) {
    match (actual, expected) {
        (Some(aqm), Some(eqm)) => match_header_maps(eqm, aqm, mismatches),
        (Some(_), None) => (),
        (None, Some(eqm)) => for (key, value) in &eqm {
            mismatches.push(Mismatch::HeaderMismatch { key: key.clone(),
                expected: format!("{:?}", value),
                actual: "".to_string(),
                mismatch: format!("Expected header '{}' but was missing", key) });
        },
        (None, None) => (),
    };
}

pub fn match_body(expected: &HttpPart, actual: &HttpPart, config: DiffConfig,
    mismatches: &mut Vec<Mismatch>) {
    if expected.mimetype() == actual.mimetype() {
        match (expected.body(), actual.body()) {
            (&OptionalBody::Missing, _) => (),
            (&OptionalBody::Null, &OptionalBody::Present(ref b)) => {
                mismatches.push(Mismatch::BodyMismatch { expected: None, actual: Some(b.clone()),
                    mismatch: format!("Expected empty body but received '{}'", b.clone()),
                    path: s!("/")});
            },
            (&OptionalBody::Null, _) => (),
            (e, &OptionalBody::Missing) => {
                mismatches.push(Mismatch::BodyMismatch { expected: Some(e.value()), actual: None,
                    mismatch: format!("Expected body '{}' but was missing", e.value()),
                    path: s!("/")});
            },
            (_, _) => {
                // if (expected.getBody.getValue == actual.getBody.getValue)
                //   List()
                // else
                //   List(BodyMismatch(expected.getBody.getValue, actual.getBody.getValue))
            }
        }
    } else if expected.body().is_present() {
        mismatches.push(Mismatch::BodyTypeMismatch { expected: expected.mimetype(),
            actual: actual.mimetype() });
    }
}

pub fn match_request(expected: Request, actual: Request) -> Vec<Mismatch> {
    let mut mismatches = vec![];

    debug!("comparing to expected request: {:?}", expected);
    match_body(&expected, &actual, DiffConfig::NO_UNEXPECTED_KEYS, &mut mismatches);
    match_method(expected.method, actual.method, &mut mismatches);
    match_path(expected.path, actual.path, &mut mismatches);
    match_query(expected.query, actual.query, &mut mismatches);
    match_headers(expected.headers, actual.headers, &mut mismatches);

    mismatches
}

pub fn match_status(expected: u16, actual: u16, mismatches: &mut Vec<Mismatch>) {
    if expected != actual {
        mismatches.push(Mismatch::StatusMismatch { expected: expected, actual: actual });
    }
}

pub fn match_response(expected: Response, actual: Response) -> Vec<Mismatch> {
    let mut mismatches = vec![];

    debug!("comparing to expected response: {:?}", expected);
    match_body(&expected, &actual, DiffConfig::ALLOW_UNEXPECTED_KEYS, &mut mismatches);
    match_status(expected.status, actual.status, &mut mismatches);
    match_headers(expected.headers, actual.headers, &mut mismatches);

    mismatches
}

#[cfg(test)]
#[macro_use(expect)]
extern crate expectest;

#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
mod tests;
