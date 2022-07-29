use actix_web::{web, HttpResponse};
use braintree::{Braintree};
use serde::{Serialize, Deserialize};
use log::{error, info};
use std::sync::{Mutex};

use crate::util;

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
#[derive(Deserialize,Debug, Serialize)]
pub struct Invoice {
    pub price : f32,
    pub invoice_id : String,
    pub due_date : Option<String>,
    pub disable_sales_tax : Option<bool>,
    pub fees : Option<f32>,
    #[serde(flatten)]
    payment : util::Payment,
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn process_invoice(
    invoice : web::Form<Invoice>,
    braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {

    let result = util::process_payment(
        &invoice.payment,
        invoice.price,
        braintree,
        util::PaymentType::Invoice,
        &format!("Invoice ID #{}", invoice.invoice_id).to_string());

    if result.is_err() {
        error!("Error: payment process {:?}\n", result);
        return util::error(util::PaymentType::Invoice).await;
    }

    info!("invoice number {} payment processed for ${}\n", invoice.invoice_id, invoice.price);

    util::thanks(util::PaymentType::Invoice).await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn invoice(braintree : web::Data<Mutex<Braintree>>, invoice : web::Query<Invoice>) -> HttpResponse {
    let braintree = braintree.lock().unwrap();

    let tax = if invoice.disable_sales_tax.unwrap_or(false) { 0.0 } else { 0.0875 };

    let tax = (invoice.price * tax) + invoice.fees.unwrap_or(0.0);

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/invoice.html")
            .replace("PRICE", &format!("{:.2}", invoice.price).to_string())
            .replace("INVOICE_ID", &invoice.invoice_id)
            .replace(
                "CLIENT_TOKEN_FROM_SERVER",
                braintree.client_token().generate(Default::default()).expect("unable to get client token").value.as_str())
            .replace("TOTAL", &format!("{:.2}", tax+invoice.price).to_string())
            .replace("TAX", &format!("{:.2}", tax).to_string()))
}

