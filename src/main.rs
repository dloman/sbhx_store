use actix_web::{web, App, HttpServer, HttpResponse};
use braintree::{Address, Braintree, CreditCard, Customer, Environment};
use std::fs::File;
use std::io::BufReader;
use log::{info};
use serde::{Serialize, Deserialize};
use std::sync::{Mutex};

#[derive(Deserialize,Debug, Serialize)]
pub struct Signup {
    pub first_name : String,
    pub last_name : String,
    pub email : String,
    pub address : String,
    pub address2 : String,
    pub city : String,
    pub state : String,
    pub payment_method_nonce : String,
    pub course_type : String,
}

#[derive(Deserialize,Debug, Serialize)]
pub struct Availability {
    pub a : i8,
    pub b : i8,
    pub c : i8,
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub fn get_availible(available: i8, class: char) -> String {
    if available >= 1 {
        return format!("<span class=\"d-block g-color-danger g-font-size-16\">{} / 16 Spaces Available</span>
        <a href=\"class{}\" class=\"w-100 btn btn-lg btn-success\" role=\"button\">Buy Now</a>", available, class);
    }
    "<span class=\"d-block g-color-danger g-font-size-16\">Sold Out</span>".to_string()
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn thanks() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/thanks.html"))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn signup(
    signup : web::Form<Signup>,
    braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {
    print!("request = {:#?}\n", signup);

    let braintree = &*(braintree.lock().unwrap());

    let result = braintree.customer().generate(Customer{
        email: Some(signup.email.to_string()),
        first_name: Some(signup.first_name.to_string()),
        last_name: Some(signup.last_name.to_string()),
        payment_method_nonce: Some(signup.payment_method_nonce.to_string()),
        credit_card: Some(CreditCard{
            billing_address: Some(Address{
                first_name: Some(signup.first_name.to_string()),
                last_name: Some(signup.last_name.to_string()),
                locality: Some(signup.city.to_string()),
                region: Some(signup.state.to_string()),
                street_address: Some(signup.address.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    });

            print!("customer {:#?}", result);
    match result {
        Ok(customer) => {
            print!("customer {:#?}", customer);
            let transaction = braintree.transaction().create(braintree::transaction::Request{
                amount: "500.00".to_string(),
                payment_method_token: customer.credit_card.unwrap().token,
                options: Some(braintree::transaction::Options{
                    submit_for_settlement: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            });


                println!("\n\ntransaction!!! {:#?} \n\n", transaction);
            match transaction {
                Ok(transaction) => println!("\n\nWooooo!!! {:#?} \n\n", transaction),
                Err(err) => println!("\nError: {}\n", err),
            }
        },
        Err(err) => println!("\nError: {}\n", err),
    }

    let file = File::open("available.json").expect("valid available.json is required");
    let reader = BufReader::new(file);
    let mut available :Availability = serde_json::from_reader(reader).expect("failure reading available.json");

    match signup.course_type.as_str() {
        "Class A" => available.a -= 1,
        "Class B" => available.b -= 1,
        "Class C" => available.c -= 1,
        _ => print!("Error: bad course type\n",),
    }

    serde_json::to_writer(&File::create("available.json").expect("unable to open file"), &available).expect("unable to write available.json");

    thanks().await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn index() -> HttpResponse {
    let file = File::open("available.json").expect("valid available.json is required");
    let reader = BufReader::new(file);
    let available :Availability = serde_json::from_reader(reader).expect("failure reading available.json");

    print!("wtf {:?}\n ", available);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/index.html")
              .replace("A_REMAIN", get_availible(available.a, 'a').as_str())
              .replace("B_REMAIN", get_availible(available.b, 'b').as_str())
              .replace("C_REMAIN", get_availible(available.c, 'c').as_str()))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn classa(braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {
    form(braintree, "Class A", 550, 50).await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn classb(braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {
    form(braintree, "Class B", 550, 50).await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn classc(braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {
    form(braintree, "Class C", 550, 50).await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn form(braintree : web::Data<Mutex<Braintree>>, course_type : &str, price : i16, discount : i16) -> HttpResponse {
    let braintree = &*(braintree.lock().unwrap());
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/form.html")
              .replace("COURSETYPE", course_type)
              .replace("PRICE", format!("{}", price).as_str())
              .replace("DISCOUNT", format!("{}", discount).as_str())
              .replace("TOTAL", format!("{}", price - discount).as_str())
              .replace("CLIENT_TOKEN_FROM_SERVER", braintree.client_token().generate(Default::default()).expect("unable to get client token").value.as_str()))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn submit(json : web::Json<serde_json::Value>) -> HttpResponse {
    HttpResponse::Ok().body(format!("submit: {:?}", &json))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    info!("setting up braintree");

    info!("starting server on 7777!");
    HttpServer::new(move || {
        let merchant_id = std::env::var("MERCHANT_ID").expect("environment variable MERCHANT_ID is not defined");
        let braintree = web::Data::new(Mutex::new(Braintree::new(
                    Environment::Sandbox,
                    merchant_id,
                    std::env::var("PUBLIC_KEY").expect("environment variable PUBLIC_KEY is not defined"),
                    std::env::var("PRIVATE_KEY").expect("environment variable PRIVATE_KEY is not defined"),
                    )));

        App::new()
            .app_data(braintree)
            .service(actix_files::Files::new("/assets", "assets").show_files_listing())
            .route("/", web::get().to(index))
            .route("/thanks", web::get().to(thanks))
            .route("/signup", web::post().to(signup))
            .route("/classa", web::get().to(classa))
            .route("/classb", web::get().to(classb))
            .route("/classc", web::get().to(classc))
            .route("/", web::post().to(submit))
    })
    .bind("0.0.0.0:7777")?
        .run()
        .await
}

