use actix_web::{web, App, HttpServer, HttpResponse};
use braintree::{Address, Braintree, CreditCard, Customer, Environment};
use log::{info};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
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

#[derive(Deserialize,Debug, Serialize, Clone)]
pub struct Item {
    pub number_of_items : Option<i32>,
    pub price : f32,
    pub discount : f32,
    pub name : String,
    pub formname : String,
    pub image : String,
}

//------------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------
impl Item {

    //--------------------------------------------------------------------------------------------------
    //--------------------------------------------------------------------------------------------------
    pub fn get_button(&self) -> String {
        match self.number_of_items {
            Some(number_of_items) => {
                if number_of_items >= 1 {
                    return format!(
                        "<span class=\"d-block g-color-danger g-font-size-16\">{} / 16 Spaces Available</span>
                         <a href=\"{}\" class=\"w-100 btn btn-lg btn-success\" role=\"button\">Buy Now</a>",
                        number_of_items,
                        self.formname);
                }
                "<span class=\"d-block g-color-danger g-font-size-16\">Sold Out</span>".to_string()
            },
            None => "<a href=\"{}\" class=\"w-100 btn btn-lg btn-success\" role=\"button\">Buy Now</a>".to_string(),
        }
    }

    //--------------------------------------------------------------------------------------------------
    //--------------------------------------------------------------------------------------------------
    pub fn get_entry(&self) -> String {

          format!("<div class=\"col-md-6 col-lg-4 g-mb-30\"><article class=\"u-shadow-v18 g-bg-white text-center rounded g-px-20 g-py-40 g-mb-5\">
            <img class=\"d-inline-block img-fluid mb-4\"  src=\"{}\" Width=100 Height=100 alt=\"Image Description\">
            <h4 class=\"h5 g-color-black g-font-weight-600 g-mb-10\">{}</h4>
            <p>Dates: July 11-15 8:00AM - 11:00am</p>
            <span class=\"d-block g-color-primary g-font-size-16\">$500.00</span>
            {}
          </article></div>", self.image, self.name, self.get_button())
    }
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
pub async fn error() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/error.html"))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn signup(
    signup : web::Form<Signup>,
    inventory : web::Data<Mutex<BTreeMap<String, Item>>>,
    braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {
    print!("request = {:#?}\n", signup);

    let inventory = &mut *(inventory.lock().unwrap());


    let item = inventory.get(&signup.course_type);

    if item.is_none() {
        return error().await;
    }

    //dont charge if no inventory available
    let item = item.unwrap();

    match item.number_of_items {
        Some(number_of_items) => {
            if number_of_items < 1 {
                return error().await;
            }
        },
        None => (),
    }

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
                amount: format!("{:.2}", item.price),
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

    match inventory.get_mut(&signup.course_type) {
        Some(item) => {
            match &mut item.number_of_items {
                Some(number_of_items) => *number_of_items -= 1,
                None => (),
            }
        },
        None => print!("Error: bad course type {}\n", signup.course_type),
    }

    print!("inventory after sale {:#?}\n", inventory);
    serde_json::to_writer(&File::create("inventory.json").expect("unable to open file"), &inventory).expect("unable to write inventory.json");

    thanks().await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn index(_inventory : web::Data<Mutex<BTreeMap<String, Item>>>) -> HttpResponse {
    let index = include_str!("../static/index.html");
    //let inventory = &*(inventory.lock().unwrap()); //i dont know why this isnt working
    let file = File::open("inventory.json").expect("valid inventory.json is required");
    let reader = BufReader::new(file);
    let inventory : BTreeMap<String, Item> = serde_json::from_reader(reader).expect("failure reading inventory.json");
    print!("inventory in index {:#?}\n", inventory);
    let mut items = String::new();
    for (_key, item) in inventory {
        items += item.get_entry().as_str();
    }

    let index = index.replace("ITEMS", &items);

    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(index)
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn item_page(braintree : web::Data<Mutex<Braintree>>, inventory : web::Data<Mutex<BTreeMap<String, Item>>>, formname : String) -> HttpResponse {
    let inventory = &*(inventory.lock().unwrap());
    form(braintree, inventory.get(&formname).unwrap()).await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn form(braintree : web::Data<Mutex<Braintree>>, item : &Item) -> HttpResponse {
    let braintree = &*(braintree.lock().unwrap());
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/form.html")
              .replace("COURSETYPE", &item.formname)
              .replace("PRICE", format!("{}", item.price + item.discount).as_str())
              .replace("DISCOUNT", format!("{}", &item.discount).as_str())
              .replace("TOTAL", format!("{}", &item.price).as_str())
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
                    Environment::from_str(&std::env::var("ENVIRONMENT").expect("environment variable ENVIRONMENT is not defined")).unwrap(),
                    merchant_id,
                    std::env::var("PUBLIC_KEY").expect("environment variable PUBLIC_KEY is not defined"),
                    std::env::var("PRIVATE_KEY").expect("environment variable PRIVATE_KEY is not defined"),
                    )));

        let file = File::open("inventory.json").expect("valid inventory.json is required");
        let reader = BufReader::new(file);
        let inventory : BTreeMap<String, Item> = serde_json::from_reader(reader).expect("failure reading inventory.json");

        let mut app = App::new()
            .app_data(braintree)
            .app_data(web::Data::new(Mutex::new(inventory.clone())))
            .service(actix_files::Files::new("/assets", "assets").show_files_listing())
            .route("/", web::get().to(index))
            .route("/thanks", web::get().to(thanks))
            .route("/error", web::get().to(error))
            .route("/signup", web::post().to(signup))
            .route("/", web::post().to(submit));

        for (_, item) in inventory {
            app = app.route(format!("/{}", item.formname).as_str(), web::get().to(
                    move |braintree, inventory| item_page(braintree, inventory, item.formname.clone())));
        }
        app
    })
    .bind("0.0.0.0:7777")?
        .run()
        .await
}

