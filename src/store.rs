use actix_web::{web, HttpResponse};
use serde::{Serialize, Deserialize};
use braintree::{Braintree};
use log::{debug, error, info};
use std::collections::BTreeMap;
use std::fs::File;
use std::sync::{Mutex};

use crate::util;

#[derive(Deserialize,Debug, Serialize)]
pub struct CourseSignup
{
    pub course_type : String,
    #[serde(flatten)]
    payment : util::Payment,
}

#[derive(Deserialize,Debug, Serialize)]
pub struct Item {
    pub number_of_items : Option<i32>,
    pub price : f32,
    pub discount : f32,
    pub name : String,
    pub formname : String,
    pub image : String,
    pub dates : String,
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
                        "<span class=\"d-block g-color-danger g-font-size-16\">{} Spaces Available</span>
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
            <p>In Person at SBHX: 5782 Thornwood Dr, Goleta, CA 93117</p>
            <p>Dates: {}</p>
            <span class=\"d-block g-color-primary g-font-size-16\">${:.2}</span>
            {}
          </article></div>", self.image, self.name, self.dates, self.price, self.get_button())
    }
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn course_signup(
    signup : web::Form<CourseSignup>,
    braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {
    debug!("course signup request = {:#?}\n", signup);

    let mut inventory = util::get_file::<BTreeMap<String, Item>>("inventory.json".to_string());

    let item = inventory.get(&signup.course_type);

    if item.is_none() {
        error!("Error: no item {} found \n", &signup.course_type);
        return util::error(util::PaymentType::CourseSignup).await;
    }

    //dont charge if no inventory available
    let item = item.unwrap();

    match item.number_of_items {
        Some(number_of_items) => {
            if number_of_items < 1 {
                error!("Error: number of items less then 0 == {:?} \n", item.number_of_items);
                return util::error(util::PaymentType::CourseSignup).await;
            }
        },
        None => (),
    }

    let result = util::process_payment(&signup.payment, item.price, braintree, util::PaymentType::CourseSignup, &item.name);
    if result.is_err() {
        error!("Error: payment process {:?}\n", result);
        return util::error(util::PaymentType::CourseSignup).await;
    }

    match inventory.get_mut(&signup.course_type) {
        Some(ref mut item) => {
            match &mut item.number_of_items {
                Some(number_of_items) => *number_of_items -= 1,
                None => (),
            }
        },
        None => {
            error!("Error: bad course type {}\n", signup.course_type);
            return util::error(util::PaymentType::CourseSignup).await;
        },
    }

    info!("inventory after course signup {:#?}\n", inventory);
    serde_json::to_writer(
        &File::create("inventory.json").expect("unable to open file"),
        &inventory).expect("unable to write inventory.json");

    util::thanks(util::PaymentType::CourseSignup).await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn store() -> HttpResponse {
    let store = include_str!("../static/store.html");
    let inventory = util::get_file::<BTreeMap<String, Item>>("inventory.json".to_string());
    info!("inventory in store {:#?}\n", inventory);
    let mut items = String::new();
    for (_key, item) in inventory.iter() {
        items += item.get_entry().as_str();
    }

    let store = store.replace("ITEMS", &items);

    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(store)
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn item_page(braintree : web::Data<Mutex<Braintree>>, formname : String) -> HttpResponse {
    let inventory = util::get_file::<BTreeMap<String, Item>>("inventory.json".to_string());
    let item = inventory.get(&formname).unwrap();
    let braintree = braintree.lock().unwrap();

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/form.html")
              .replace("COURSETYPE", &item.formname)
              .replace("PRICE", format!("{}", item.price + item.discount).as_str())
              .replace("DISCOUNT", format!("{}", &item.discount).as_str())
              .replace("TOTAL", format!("{}", &item.price).as_str())
              .replace(
                  "CLIENT_TOKEN_FROM_SERVER",
                  braintree.client_token().generate(Default::default()).expect("unable to get client token").value.as_str()))
}

