use actix_web::{web, App, HttpServer, HttpResponse};
use braintree::{Address, Braintree, CreditCard, Customer, Environment};
use log::{debug, error, info};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Mutex};


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
}

#[derive(Deserialize,Debug, Serialize)]
pub struct CourseSignup
{
    pub course_type : String,
    #[serde(flatten)]
    payment : Payment,
}

#[derive(Deserialize,Debug, Serialize)]
pub struct Donation
{
    pub amount : f32,
    pub fundraiser_name : String,
    #[serde(flatten)]
    payment : Payment,
}

#[derive(Deserialize,Debug, Serialize, Clone, Default)]
pub struct Item {
    pub number_of_items : Option<i32>,
    pub price : f32,
    pub discount : f32,
    pub name : String,
    pub formname : String,
    pub image : String,
    pub dates : String,
}

#[derive(Deserialize,Debug, Serialize, Clone, Default)]
pub struct Fundraiser {
    pub name : String,
    pub goal : f32,
    pub amount_raised : f32,
    pub formname : String,
    pub image : String,
    pub description : String,
}

#[derive(Deserialize,Debug, Serialize)]
pub struct Quote {
    pub price : f32,
    pub invoice_id : String,
    pub company_name : String,
    pub date : String,
}

enum PaymentType {
    CourseSignup,
    Donation
}

impl PaymentType {
    fn as_str(&self) -> &'static str {
         match self {
            //PaymentType::Quote => "Quote",
            PaymentType::CourseSignup => "Course Signup",
            PaymentType::Donation => "Donation"
        }
    }

    fn get_url(&self) -> &'static str {
         match self {
            //PaymentType::Quote => "Quote",
            PaymentType::CourseSignup => "https://store.sbhackerspace.com",
            PaymentType::Donation => "https://donate.sbhackerspace.com",
        }
    }
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
impl Fundraiser {
    //--------------------------------------------------------------------------------------------------
    //--------------------------------------------------------------------------------------------------
    pub fn get_button(&self) -> String {
        return format!(
            "<a href=\"{}\" class=\"w-50 btn btn-lg btn-success\" role=\"button\">Donate Now</a>",
            self.formname);
    }

    //--------------------------------------------------------------------------------------------------
    //--------------------------------------------------------------------------------------------------
    pub fn get_entry(&self) -> String {

          format!("<div class=\"col-md-6 col-lg-4 g-mb-30\"><article class=\"u-shadow-v18 g-bg-white text-center rounded g-px-20 g-py-40 g-mb-5\">
            <img class=\"d-inline-block img-fluid mb-4\" Width=\"400\" Height=\"200\" src=\"{}\" alt=\"Image Description\">
            <h4 class=\"h5 g-color-black g-font-weight-600 g-mb-10\">{}</h4>
            <p> {} </p>
            <div class=\"progress\">
              <div class=\"progress-bar bg-success\" role=\"progressbar\" style=\"width: {}%\" aria-valuenow=\"{}\" aria-valuemin=\"0\" aria-valuemax=\"{}\">${} of ${} Raised</div>
            </div>
            <p>  </p>
            {}
            </article></div>",
          self.image,
          self.name,
          self.description,
          (100.0* (1.0- (self.goal - self.amount_raised)/self.goal)) as i32,
          self.amount_raised as i32,
          self.goal as i32,
          self.amount_raised as i32,
          self.goal as i32,
          self.get_button())
    }

}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
async fn thanks(payment_type: PaymentType) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/thanks.html")
            .replace("NAME", payment_type.as_str())
            .replace("URL", payment_type.get_url()))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
async fn error(payment_type: PaymentType) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/error.html")
            .replace("NAME", payment_type.as_str())
            .replace("URL", payment_type.get_url()))


}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
fn process_payment(payment : &Payment, price: f32, braintree : web::Data<Mutex<Braintree>>, payment_type: PaymentType, description: &String) -> Result<braintree::transaction::Transaction, braintree::Error>{
    let braintree = braintree.lock().unwrap();

    debug!("trying to generate customer\n");
    let result = braintree.customer().generate(Customer{
        email: Some(payment.email.to_string()),
        first_name: Some(payment.first_name.to_string()),
        last_name: Some(payment.last_name.to_string()),
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
pub async fn process_donation(
    donation : web::Form<Donation>,
    fundraisers : web::Data<Mutex<BTreeMap<String, Fundraiser>>>,
    braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {

    let mut fundraisers = fundraisers.lock().unwrap();

    let result = process_payment(&donation.payment, donation.amount, braintree, PaymentType::Donation, &donation.fundraiser_name);

    if result.is_err() {
        error!("Error: payment process {:?}\n", result);
        return error(PaymentType::Donation).await;
    }

    info!("donation of {} processed for {}\n",donation.amount, donation.fundraiser_name);

    match fundraisers.get_mut(&donation.fundraiser_name) {
        Some(fundraiser) => {
            fundraiser.amount_raised += donation.amount;
        },
        None => {
            error!("Error: unknown fundraiser name {}\n", donation.fundraiser_name);
            return error(PaymentType::Donation).await;
        },
    }

    thanks(PaymentType::Donation).await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn course_signup(
    signup : web::Form<CourseSignup>,
    inventory : web::Data<Mutex<BTreeMap<String, Item>>>,
    braintree : web::Data<Mutex<Braintree>>) -> HttpResponse {
    debug!("course signup request = {:#?}\n", signup);

    let mut inventory = inventory.lock().unwrap();

    let item = inventory.get(&signup.course_type);

    if item.is_none() {
        error!("Error: no item {} found \n", &signup.course_type);
        return error(PaymentType::CourseSignup).await;
    }

    //dont charge if no inventory available
    let item = item.unwrap();

    match item.number_of_items {
        Some(number_of_items) => {
            if number_of_items < 1 {
                error!("Error: number of items less then 0 == {:?} \n", item.number_of_items);
                return error(PaymentType::CourseSignup).await;
            }
        },
        None => (),
    }

    let result = process_payment(&signup.payment, item.price, braintree, PaymentType::CourseSignup, &item.name);
    if result.is_err() {
        error!("Error: payment process {:?}\n", result);
        return error(PaymentType::CourseSignup).await;
    }

    match inventory.get_mut(&signup.course_type) {
        Some(item) => {
            match &mut item.number_of_items {
                Some(number_of_items) => *number_of_items -= 1,
                None => (),
            }
        },
        None => {
            error!("Error: bad course type {}\n", signup.course_type);
            return error(PaymentType::CourseSignup).await;
        },
    }

    info!("inventory after course signup {:#?}\n", inventory);
    serde_json::to_writer(
        &File::create("inventory.json").expect("unable to open file"),
        &*inventory).expect("unable to write inventory.json");

    thanks(PaymentType::CourseSignup).await
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn fundraisers(fundraising_goals : web::Data<Mutex<BTreeMap<String, Fundraiser>>>)-> HttpResponse {
    let web_page = include_str!("../static/fundraise.html");
    let fundraising_goals = fundraising_goals.lock().unwrap();

    let mut content = String::new();
    for (_, fundraiser) in fundraising_goals.iter() {
        content += fundraiser.get_entry().as_str();
    }
    let web_page = web_page.replace("FUNDRAISERS", &content);

    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(web_page)
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn store(inventory : web::Data<Mutex<BTreeMap<String, Item>>>) -> HttpResponse {
    let store = include_str!("../static/store.html");
    let inventory = inventory.lock().unwrap();
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
pub async fn item_page(braintree : web::Data<Mutex<Braintree>>, inventory : web::Data<Mutex<BTreeMap<String, Item>>>, formname : String) -> HttpResponse {
    let inventory = inventory.lock().unwrap();
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

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn fundraiser_page(braintree : web::Data<Mutex<Braintree>>, fundraisers : web::Data<Mutex<BTreeMap<String, Fundraiser>>>, name : String) -> HttpResponse {
    let fundraisers = fundraisers.lock().unwrap();
    let fundraiser = fundraisers.get(&name).unwrap();
    let braintree = braintree.lock().unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/donate.html")
              .replace("FORMNAME", &fundraiser.formname)
              .replace("NAME", &fundraiser.name)
              .replace(
                  "CLIENT_TOKEN_FROM_SERVER",
                  braintree.client_token().generate(Default::default()).expect("unable to get client token").value.as_str()))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn quote(braintree : web::Data<Mutex<Braintree>>, quote : web::Query<Quote>) -> HttpResponse {
    let braintree = braintree.lock().unwrap();
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("../static/quote.html")
              .replace("PRICE", &format!("{:.2}", quote.price).to_string())
              .replace("INVOICE_ID", &quote.invoice_id)
              .replace("COMPANY_NAME", &quote.company_name)
              .replace("DATE", &quote.date)
              .replace(
                  "CLIENT_TOKEN_FROM_SERVER",
                  braintree.client_token().generate(Default::default()).expect("unable to get client token").value.as_str()))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub async fn submit(json : web::Json<serde_json::Value>) -> HttpResponse {
    HttpResponse::Ok().body(format!("submit: {:?}", &json))
}

//----------------------------------------------------------------------------------------------------
//----------------------------------------------------------------------------------------------------
pub fn get_file<T: serde::de::DeserializeOwned>(file_name: String) -> web::Data<Mutex<T>>
{
    let file = File::open(&file_name).expect(format!("unable to open {:}", &file_name.as_str()).as_str());
    let reader = BufReader::new(file);
    let data :T = serde_json::from_reader(reader).expect("failure reading inventory.json");
    web::Data::new(Mutex::new(data))
}

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

        let inventory = get_file::<BTreeMap<String, Item>>("inventory.json".to_string());

        let item_names :Vec<String> = inventory.lock().unwrap().keys().map(|x| String::clone(x)).collect();

        let fundraising_goals = get_file::<BTreeMap<String, Fundraiser>>("fundraising_goals.json".to_string());

        let fundraiser_names : Vec<String> = fundraising_goals.lock().unwrap().keys().map(|x| String::clone(x)).collect();

        let mut app = App::new()
            .app_data(braintree)
            .app_data(inventory.clone())
            .app_data(fundraising_goals.clone())
            .service(actix_files::Files::new("/assets", "assets").show_files_listing())
            .route("/", web::get().to(store))
            .route("/store", web::get().to(store))
            .route("/process_donation", web::post().to(process_donation))
            .route("/signup", web::post().to(course_signup))
            .route("/quote", web::get().to(quote))
            .route("/fundraise", web::get().to(fundraisers))
            .route("/donate", web::get().to(fundraisers))
            .route("/", web::post().to(submit));

        for fundraiser_name in fundraiser_names.into_iter() {
            app = app.route(format!("/{}", fundraiser_name).as_str(), web::get().to(
                    move |braintree, fundraisers| fundraiser_page(braintree, fundraisers, fundraiser_name.clone())));
        }

        for item_name in item_names.into_iter() {
            app = app.route(format!("/{}", item_name).as_str(), web::get().to(
                    move |braintree, current_inventory| item_page(braintree, current_inventory, item_name.clone())));
        }
        app
    })
    .bind("0.0.0.0:7777")?
        .run()
        .await
}
