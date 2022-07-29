use actix_web::{web, HttpResponse};
use serde::{Serialize, Deserialize};
use log::{debug};
use std::collections::HashMap;
use std::sync::{Mutex};
use std::fs::File;
use std::io::BufReader;
use braintree::{Address, Braintree, CreditCard, Customer};

#[derive(Deserialize,Debug, Serialize)]
pub struct Payment {
    pub first_name : String,
    pub last_name : String,
    pub email : String,
    pub address : String,
    pub address2 : String,
    pub city : String,
    pub state : String,
    pub payment_method_nonce : String,
    pub company_name : Option<String>,
}

pub enum PaymentType {
    CourseSignup,
    Donation,
    Invoice
}

impl PaymentType {
    fn as_str(&self) -> &'static str {
         match self {
            PaymentType::CourseSignup => "Course Signup",
            PaymentType::Donation => "Donation",
            PaymentType::Invoice => "Invoice"
        }
    }

    fn get_url(&self) -> &'static str {
         match self {
            PaymentType::CourseSignup => "https://store.sbhackerspace.com",
            PaymentType::Donation => "https://donate.sbhackerspace.com",
            PaymentType::Invoice => "https://invoice.sbhackerspace.com",
        }
    }
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn thanks(payment_type: PaymentType) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/thanks.html")
            .replace("NAME", payment_type.as_str())
            .replace("URL", payment_type.get_url()))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn error(payment_type: PaymentType) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/error.html")
            .replace("NAME", payment_type.as_str())
            .replace("URL", payment_type.get_url()))


}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub fn process_payment(payment : &Payment, price: f32, braintree : web::Data<Mutex<Braintree>>, payment_type: PaymentType, description: &String) -> Result<braintree::transaction::Transaction, braintree::Error>{
    let braintree = braintree.lock().unwrap();

    debug!("trying to generate customer\n");
    let result = braintree.customer().generate(Customer{
        email: Some(payment.email.to_string()),
        first_name: Some(payment.first_name.to_string()),
        last_name: Some(payment.last_name.to_string()),
        company: payment.company_name.clone(),
        payment_method_nonce: Some(payment.payment_method_nonce.to_string()),
        credit_card: Some(CreditCard{
            billing_address: Some(Address{
                first_name: Some(payment.first_name.to_string()),
                last_name: Some(payment.last_name.to_string()),
                locality: Some(payment.city.to_string()),
                region: Some(payment.state.to_string()),
                street_address: Some(payment.address.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    });

    debug!("customer = {:?}\n", result);
    match result {
        Ok(customer) => {
            braintree.transaction().create(braintree::transaction::Request{
                amount: format!("{:.2}", price),
                payment_method_token: customer.credit_card.unwrap().token,
                options: Some(braintree::transaction::Options{
                    submit_for_settlement: Some(true),
                    ..Default::default()
                }),
                descriptor: Some(braintree::descriptor::Descriptor{
                    name: Some("sbhx   *   product".to_string()),
                    url: Some("".to_string()),
                    phone: Some("8052422533".to_string()),
                }),
                custom_fields: HashMap::from([("payment_type".to_string(), payment_type.as_str().to_string()), ("description".to_string(), description.clone())]),
                ..Default::default()
            })
        },
        Err(error) => Err(error),
    }
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub fn get_file<T: serde::de::DeserializeOwned>(file_name: String) -> T
{
    let file = File::open(&file_name).expect(format!("unable to open {:}", &file_name.as_str()).as_str());
    let reader = BufReader::new(file);
    let data :T = serde_json::from_reader(reader).expect("failure reading inventory.json");
    data
}

