use actix_web::{web, App, HttpServer};
use braintree::{Braintree, Environment};
use log::{info};
use std::collections::BTreeMap;
use std::sync::{Mutex};

pub mod util;
pub mod quote;
pub mod fundraise;
pub mod store;
//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("setting up braintree");

    info!("starting server on 7777!");

    HttpServer::new(move || {
        let braintree = web::Data::new(Mutex::new(Braintree::new(
                    Environment::from_str(&std::env::var("ENVIRONMENT").expect("environment variable ENVIRONMENT is not defined")).unwrap(),
                    std::env::var("MERCHANT_ID").expect("environment variable MERCHANT_ID is not defined"),
                    std::env::var("PUBLIC_KEY").expect("environment variable PUBLIC_KEY is not defined"),
                    std::env::var("PRIVATE_KEY").expect("environment variable PRIVATE_KEY is not defined"),
                    )));

        let inventory = util::get_file::<BTreeMap<String, store::Item>>("inventory.json".to_string());

        let item_names :Vec<String> = inventory.keys().map(|x| String::clone(x)).collect();

        let fundraisers = util::get_file::<BTreeMap<String, fundraise::Fundraiser>>("fundraising_goals.json".to_string());

        let fundraiser_names : Vec<String> = fundraisers.keys().map(|x| String::clone(x)).collect();

        let mut app = App::new()
            .app_data(braintree)
            .service(actix_files::Files::new("/assets", "assets").show_files_listing())
            .route("/store", web::get().to(store::store))
            .route("/store/signup", web::post().to(store::course_signup))
            .route("/quote/process_invoice", web::post().to(quote::process_invoice))
            .route("/quote/invoice", web::get().to(quote::invoice))
            .route("/donate/process_donation", web::post().to(fundraise::process_donation))
            .route("/donate/fundraise", web::get().to(fundraise::fundraisers_page))
            .route("/donate", web::get().to(fundraise::fundraisers_page));

        for fundraiser_name in fundraiser_names.into_iter() {
            app = app.route(format!("/donate/{}", fundraiser_name).as_str(), web::get().to(
                    move |braintree| fundraise::fundraiser_page(braintree, fundraiser_name.clone())));
        }

        for item_name in item_names.into_iter() {
            app = app.route(format!("/store/{}", item_name).as_str(), web::get().to(
                    move |braintree| store::item_page(braintree, item_name.clone())));
        }
        app
    })
    .bind("0.0.0.0:7777")?
        .run()
        .await
}
