//! # paypal-rs
//! ![Rust](https://github.com/edg-l/paypal-rs/workflows/Rust/badge.svg)
//! ![Docs](https://docs.rs/paypal-rs/badge.svg)
//!
//! A rust library that wraps the [paypal api](https://developer.paypal.com/docs/api) asynchronously in a strongly typed manner.
//!
//! Crate: https://crates.io/crates/paypal-rs
//!
//! Documentation: https://docs.rs/paypal-rs
//!
//! Currently in early development.

#![deny(missing_docs)]

pub mod common;
pub mod errors;
pub mod invoice;
pub mod orders;

use reqwest::header;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// The paypal api endpoint used on a live application.
pub const LIVE_ENDPOINT: &str = "https://api-m.paypal.com";
/// The paypal api endpoint used on when testing.
pub const SANDBOX_ENDPOINT: &str = "https://api-m.sandbox.paypal.com";

/// Represents the access token returned by the OAuth2 authentication.
///
/// https://developer.paypal.com/docs/api/get-an-access-token-postman/
#[derive(Debug, Deserialize)]
pub struct AccessToken {
    /// The OAuth2 scopes.
    pub scope: String,
    /// The access token.
    pub access_token: String,
    /// The token type.
    pub token_type: String,
    /// The app id.
    pub app_id: String,
    /// Seconds until it expires.
    pub expires_in: u64,
    /// The nonce.
    pub nonce: String,
}

/// Stores OAuth2 information.
#[derive(Debug)]
pub struct Auth {
    /// Your client id.
    pub client_id: String,
    /// The secret.
    pub secret: String,
    /// The access token returned by oauth2 authentication.
    pub access_token: Option<AccessToken>,
    /// Used to check when the token expires.
    pub expires: Option<(Instant, Duration)>,
}

/// Represents a client used to interact with the paypal api.
#[derive(Debug)]
pub struct Client {
    /// Internal http client
    pub client: reqwest::Client,
    /// Whether you are or not in a sandbox enviroment.
    pub sandbox: bool,
    /// Api Auth information
    pub auth: Auth,
}

/// Represents the query used in most GET api requests.
///
/// Reference: https://developer.paypal.com/docs/api/reference/api-requests/#query-parameters
///
/// Note: You can avoid most fields by the Default impl like so:
/// ```
/// use paypal_rs::Query;
/// let query = Query { count: Some(40), ..Default::default() };
/// ```
#[derive(Debug, Default, Serialize)]
pub struct Query {
    /// The number of items to list in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    /// The end date and time for the range to show in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    /// The page number indicating which set of items will be returned in the response.
    /// So, the combination of page=1 and page_size=20 returns the first 20 items.
    /// The combination of page=2 and page_size=20 returns items 21 through 40.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    /// The number of items to return in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    /// Indicates whether to show the total count in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count_required: Option<bool>,
    /// Sorts the payments in the response by a specified value, such as the create time or update time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,
    /// Sorts the items in the response in ascending or descending order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<String>,
    /// The ID of the starting resource in the response.
    /// When results are paged, you can use the next_id value as the start_id to continue with the next set of results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_id: Option<String>,
    /// The start index of the payments to list. Typically, you use the start_index to jump to a specific position in the resource history based on its cart.
    /// For example, to start at the second item in a list of results, specify start_index=2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i32>,
    /// The start date and time for the range to show in the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    // TODO: Use https://github.com/samscott89/serde_qs
}

/// The preferred server response upon successful completion of the request.
#[derive(Debug, Eq, PartialEq)]
pub enum Prefer {
    /// The server returns a minimal response to optimize communication between the API caller and the server.
    /// A minimal response includes the id, status and HATEOAS links.
    Minimal,
    /// The server returns a complete resource representation, including the current state of the resource.
    Representation,
}

impl Default for Prefer {
    fn default() -> Self {
        Prefer::Minimal
    }
}

/// Represents the optional header values used on paypal requests.
///
/// https://developer.paypal.com/docs/api/reference/api-requests/#paypal-auth-assertion
#[derive(Debug, Default)]
pub struct HeaderParams {
    /// The merchant payer id used on PayPal-Auth-Assertion
    pub merchant_payer_id: Option<String>,
    /// Verifies that the payment originates from a valid, user-consented device and application.
    /// Reduces fraud and decreases declines. Transactions that do not include a client metadata ID are not eligible for PayPal Seller Protection.
    pub client_metadata_id: Option<String>,
    /// Identifies the caller as a PayPal partner. To receive revenue attribution, specify a unique build notation (BN) code.
    /// BN codes provide tracking on all transactions that originate or are associated with a particular partner.
    pub partner_attribution_id: Option<String>,
    /// Contains a unique user-generated ID that the server stores for a period of time. Use this header to enforce idempotency on REST API POST calls.
    /// You can make these calls any number of times without concern that the server creates or completes an action on a resource more than once.
    /// You can retry calls that fail with network timeouts or the HTTP 500 status code. You can retry calls for as long as the server stores the ID.
    pub request_id: Option<String>,
    /// The preferred server response upon successful completion of the request.
    pub prefer: Option<Prefer>,
    /// The media type. Required for operations with a request body.
    pub content_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuthAssertionClaims {
    pub iss: String,
    pub payer_id: String,
}

impl Client {
    /// Returns a new client, you must get_access_token afterwards to interact with the api.
    ///
    /// # Examples
    ///
    /// ```
    /// use paypal_rs::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     # dotenv::dotenv().ok();
    ///     let clientid = std::env::var("PAYPAL_CLIENTID").unwrap();
    ///     let secret = std::env::var("PAYPAL_SECRET").unwrap();
    ///
    ///     let mut client = Client::new(
    ///         clientid,
    ///         secret,
    ///         true,
    ///     );
    ///     client.get_access_token().await.unwrap();
    /// }
    /// ```
    pub fn new<S: Into<String>>(client_id: S, secret: S, sandbox: bool) -> Client {
        Client {
            client: reqwest::Client::new(),
            sandbox,
            auth: Auth {
                client_id: client_id.into(),
                secret: secret.into(),
                access_token: None,
                expires: None,
            },
        }
    }

    fn endpoint(&self) -> &str {
        if self.sandbox {
            SANDBOX_ENDPOINT
        } else {
            LIVE_ENDPOINT
        }
    }

    /// Sets up the request headers as required on https://developer.paypal.com/docs/api/reference/api-requests/#http-request-headers
    async fn setup_headers(
        &mut self,
        builder: reqwest::RequestBuilder,
        header_params: HeaderParams,
    ) -> reqwest::RequestBuilder {
        // Check if the token hasn't expired here, since it's called before any other call.
        if let Err(e) = self.get_access_token().await {
            log::warn!(target: "paypal-rs", "error getting access token: {:?}", e);
        }

        let mut headers = HeaderMap::new();

        headers.append(header::ACCEPT, "application/json".parse().unwrap());

        if let Some(token) = &self.auth.access_token {
            headers.append(
                header::AUTHORIZATION,
                format!("Bearer {}", token.access_token).parse().unwrap(),
            );
        }

        if let Some(merchant_payer_id) = header_params.merchant_payer_id {
            let claims = AuthAssertionClaims {
                iss: self.auth.client_id.clone(),
                payer_id: merchant_payer_id,
            };
            let jwt_header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
            let token = jsonwebtoken::encode(
                &jwt_header,
                &claims,
                &jsonwebtoken::EncodingKey::from_secret(self.auth.secret.as_ref()),
            )
            .unwrap();
            let encoded_token = base64::encode(token);
            headers.append("PayPal-Auth-Assertion", encoded_token.parse().unwrap());
        }

        if let Some(client_metadata_id) = header_params.client_metadata_id {
            headers.append("PayPal-Client-Metadata-Id", client_metadata_id.parse().unwrap());
        }

        if let Some(partner_attribution_id) = header_params.partner_attribution_id {
            headers.append("PayPal-Partner-Attribution-Id", partner_attribution_id.parse().unwrap());
        }

        if let Some(request_id) = header_params.request_id {
            headers.append("PayPal-Request-Id", request_id.parse().unwrap());
        }

        if let Some(prefer) = header_params.prefer {
            match prefer {
                Prefer::Minimal => headers.append("Prefer", "return=minimal".parse().unwrap()),
                Prefer::Representation => headers.append("Prefer", "return=representation".parse().unwrap()),
            };
        }

        if let Some(content_type) = header_params.content_type {
            headers.append(header::CONTENT_TYPE, content_type.parse().unwrap());
        }

        builder.headers(headers)
    }

    /// Gets a access token used in all the api calls.
    pub async fn get_access_token(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.access_token_expired() {
            return Ok(());
        }
        let res = self
            .client
            .post(format!("{}/v1/oauth2/token", self.endpoint()).as_str())
            .basic_auth(&self.auth.client_id, Some(&self.auth.secret))
            .header("Content-Type", "x-www-form-urlencoded")
            .header("Accept", "application/json")
            .body("grant_type=client_credentials")
            .send()
            .await?;

        if res.status().is_success() {
            let token = res.json::<AccessToken>().await?;
            self.auth.expires = Some((Instant::now(), Duration::new(token.expires_in, 0)));
            self.auth.access_token = Some(token);
            Ok(())
        } else {
            Err(Box::new(res.json::<errors::ApiResponseError>().await?))
        }
    }

    /// Checks if the access token expired.
    pub fn access_token_expired(&self) -> bool {
        if let Some(expires) = self.auth.expires {
            expires.0.elapsed() >= expires.1
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{orders::*, Client, HeaderParams, Prefer};
    use crate::common::Currency;
    use std::str::FromStr;
    use std::env;

    async fn create_client() -> Client {
        dotenv::dotenv().ok();
        let clientid = env::var("PAYPAL_CLIENTID").unwrap();
        let secret = env::var("PAYPAL_SECRET").unwrap();

        let client = Client::new(clientid, secret, true);

        client
    }

    #[tokio::test]
    async fn test_order() {
        let mut client = create_client().await;

        let order = OrderPayload::new(Intent::Authorize, vec![PurchaseUnit::new(Amount::new(Currency::EUR, "10.0"))]);

        let ref_id = format!(
            "TEST-{:?}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        let order_created = client
            .create_order(
                order,
                HeaderParams {
                    prefer: Some(Prefer::Representation),
                    request_id: Some(ref_id.clone()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_ne!(order_created.id, "");
        assert_eq!(order_created.status, OrderStatus::Created);
        assert_eq!(order_created.links.len(), 4);

        client
            .update_order(
                order_created.id,
                Some(Intent::Capture),
                Some(order_created.purchase_units.expect("to exist")),
            )
            .await
            .unwrap();
    }

    #[test]
    fn test_currency() {
        assert_eq!(Currency::EUR.to_string(), "EUR");
        assert_eq!(Currency::JPY.to_string(), "JPY");
        assert_eq!(Currency::JPY, Currency::from_str("JPY").unwrap());
    }
}
