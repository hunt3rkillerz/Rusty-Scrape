extern crate reqwest;
extern crate select;
extern crate rand;
extern crate csv;

use std::vec::Vec;
use std::env;
use std::fs::File;
use std::io::Read;
use rand::{thread_rng, Rng};
use rand::seq::SliceRandom;
use select::document::Document;
use select::predicate::{Predicate, Attr, Name};
use reqwest::header::USER_AGENT;
use std::time::Duration;
use rayon::prelude::*;
use clap::{Arg, App};

fn main() {
    let mut matches = App::new("Rusty Scrape")
        .version("1.0")
        .author("Hunt3rKillerZ https://github.com/hunt3rkillerz/")
        .about("A tool for pentesters to generate targeted email lists for password sprays and other use cases. DO NOT USE FOR EVIL!")
        .arg(Arg::new("CompanyName")
            .about("Specifies the name of the company to scan for.")
            .required(true)
            .index(1))
        .arg(Arg::with_name("proxy")
            .short('p')
            .long("proxy")
            .value_name("Proxy")
            .about("Use this flag to scan via proxies. If set to true the scan WILL take much longer.")
            .takes_value(false))
        .arg(Arg::with_name("wordlist")
            .short('w')
            .long("wordlist")
            .value_name("WordList")
            .about("Path to a wordlist file which can be used for the scan.")
            .takes_value(true))
        .get_matches();

    let res = splitVector(matches.is_present("proxy"), matches.value_of("CompanyName").unwrap(), getWordList(matches.value_of("wordlist").unwrap_or("wordlists/default.csv")));
    println!("\nScan Finished:\n");
    println!("{:#?}", res);
}

fn getWordList(fileLoc: &str) -> Vec<String> {
    let mut dirPath = env::current_dir().unwrap();
    dirPath.push(fileLoc.to_string());
    if dirPath.exists(){
        let mut file = File::open(dirPath).unwrap();
        let mut fileData = String::new();
        file.read_to_string(&mut fileData).unwrap();
        let mut retVec: Vec<String> = vec![];
        for value in fileData.split(","){
            retVec.push(value.to_string());
        }
        return retVec;
    }
    else {
        panic!("Provided Wordlist path does not exist");
    }
}

fn processBingData(doc: Document) -> Vec<Vec<String>> {
    let mut userVec = Vec::new();
    for node in doc.find(Name("li")) {
        let mut part1 = match node.find(Name("h2")).next() {
            None => continue,
            Some(data) => {
                data
            },
        };
        // Get the relevant text
        let relData = match part1.find(Name("a")).next() {
            None => continue,
            Some(data) => {
                data.text()
            },
        };

        let mut tokens: Vec<&str> = relData.split("-").collect();
        if tokens.len() < 3 {
            // Wrong type of result
            continue
        }
        
        // Remove trailing space
        let mut nameData = tokens[0].to_string();
        nameData.pop();

        // Remove trailing & leading space
        let mut job = &tokens[1][1..tokens[1].len()-1];

        let nameTokens: Vec<&str> = nameData.split(" ").collect();
        
        if nameTokens.len() < 2 {
            continue
        }

        userVec.push(vec![nameTokens[0].to_string(), nameTokens[1].to_string(), job.to_string()]);
    }
    // Debug
    //println!("Returning Data: {:?}", userVec);
    return userVec;
}

fn scrape(prof: &str, company_name: &str, mut proxy: Option<Vec<String>>) -> Vec<Vec<String>> {
    let mut isProxy = false;
    let mut proxy_list = match proxy {
        Some(proxy_list) => {
            isProxy = true;
            proxy_list
        }
        None => vec![]
    };
    loop {
        let searchURL = format!("http://www.bing.com/search?q=%22{}%22+%22{}%22+site%3Alinkedin.com",
                                        &prof, &company_name);
        // Early Declaration
        let mut client;
        // Only go through a proxy if the list is provided
        if isProxy {
            if proxy_list.len() == 0 {
                println!("We really ran outta proxies");
                proxy_list = fetchProxyList();
            }
            let proxy = findProxy(&mut proxy_list);

            client = reqwest::blocking::Client::builder().proxy(
                reqwest::Proxy::all(&proxy).unwrap()
            ).timeout(
                // 15 Second Timeout because this is supposed to work
                Duration::new(20, 0)
            ).build().unwrap();
        }
        else {
            client = reqwest::blocking::Client::builder().
            timeout(
                // 15 Second Timeout because this is supposed to work
                Duration::new(20, 0)
            ).build().unwrap();
        }

        let res = match client.get(&searchURL)
                .header(USER_AGENT, getRandomUserAgent())
                .send(){
                    Err(e) => {
                        continue
                    },
                    Ok(res) => res
        };  
        //println!("RESP DATA {:?}", res);
        
        let mut doc = match Document::from_read(res) {
            Err(e) => continue,
            Ok(doc) => doc
        };
        if doc.find(Name("li")).count() < 8 && isProxy {
            println!("BLOCKER");
            continue
        }
        return processBingData(doc);
    }
}

fn splitVector(useProxy: bool, company_name: &str, professionList: Vec<String>) -> Vec<Vec<String>> {
    let mut proxy_list = fetchProxyList();
    let mut data: Vec<Vec<Vec<String>>>;
    if useProxy {
        data = professionList[..7].par_iter()
            .map(|i| scrape(&i, &company_name.clone(), Some(proxy_list.clone())))
            .collect();
    }
    else {
        data = professionList[..7].par_iter() 
            .map(|i| scrape(&i, &company_name.clone(), None))
            .collect();
    }
    let mut cleanArr = Vec::new();
    for arr in data {
        for elem in arr {
            cleanArr.push(elem);
        }
    }
    //println!("\n\n\n");
    //println!("This is the final Data: {:?}", cleanArr);
    return cleanArr;
}

fn findProxy(proxy_list: &mut Vec<String>) -> String {
    let mut rng = thread_rng();
    while proxy_list.len() > 0 {
        // Select a proxy from the list at random and pop it
        let randVal = rng.gen_range(0, proxy_list.len()-1);
        let proxy = proxy_list[randVal].clone();
        proxy_list.remove(randVal);
        const url: &str = "http://www.bing.com/search?q=Test+Search&qs=n&form=QBRE&sp=-1&ghc=1&pq=test+searc&sc=5-10&sk=&cvid=B580BE8B2CDB4AB7817D09B5011F1A6C";
        let client = reqwest::blocking::Client::builder().proxy(
            reqwest::Proxy::all(&proxy).unwrap()
        ).timeout(
            // 10 Second Timeout means basically everything works
            Duration::new(10, 0)
        ).build().unwrap();
        let res = match client.get(url)
            .header(USER_AGENT, getRandomUserAgent())
            .send()
        {
            Err(e) => {
                continue
            },
            Ok(res) => res
        };  

        // We found our boi
        return proxy.to_string();
    }
    // Only here because this method is disgusting
    return "".to_string();
}
fn getRandomUserAgent() -> String {
    let user_agent_list = [
        //Chrome
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.113 Safari/537.36",
        "Mozilla/5.0 (Windows NT 6.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36",
        "Mozilla/5.0 (Windows NT 5.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36",
        "Mozilla/5.0 (Windows NT 6.2; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.90 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/44.0.2403.157 Safari/537.36",
        "Mozilla/5.0 (Windows NT 6.3; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/60.0.3112.113 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/57.0.2987.133 Safari/537.36",
        "Mozilla/5.0 (Windows NT 6.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/57.0.2987.133 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/55.0.2883.87 Safari/537.36",
        "Mozilla/5.0 (Windows NT 6.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/55.0.2883.87 Safari/537.36",
        //Firefox
        "Mozilla/4.0 (compatible; MSIE 9.0; Windows NT 6.1)",
        "Mozilla/5.0 (Windows NT 6.1; WOW64; Trident/7.0; rv:11.0) like Gecko",
        "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; WOW64; Trident/5.0)",
        "Mozilla/5.0 (Windows NT 6.1; Trident/7.0; rv:11.0) like Gecko",
        "Mozilla/5.0 (Windows NT 6.2; WOW64; Trident/7.0; rv:11.0) like Gecko",
        "Mozilla/5.0 (Windows NT 10.0; WOW64; Trident/7.0; rv:11.0) like Gecko",
        "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.0; Trident/5.0)",
        "Mozilla/5.0 (Windows NT 6.3; WOW64; Trident/7.0; rv:11.0) like Gecko",
        "Mozilla/5.0 (compatible; MSIE 9.0; Windows NT 6.1; Trident/5.0)",
        "Mozilla/5.0 (Windows NT 6.1; Win64; x64; Trident/7.0; rv:11.0) like Gecko",
        "Mozilla/5.0 (compatible; MSIE 10.0; Windows NT 6.1; WOW64; Trident/6.0)",
        "Mozilla/5.0 (compatible; MSIE 10.0; Windows NT 6.1; Trident/6.0)",
        "Mozilla/4.0 (compatible; MSIE 8.0; Windows NT 5.1; Trident/4.0; .NET CLR 2.0.50727; .NET CLR 3.0.4506.2152; .NET CLR 3.5.30729)"
    ];
    user_agent_list.choose(&mut rand::thread_rng()).unwrap().to_string()
}

fn fetchProxyList() -> Vec<String> {
    
    // Randomize user agent

    // Make Request
    const url: &str = "https://www.sslproxies.org/";
    let client = reqwest::blocking::Client::new();
    let res = client.get(url)
        .header(USER_AGENT, getRandomUserAgent())
        .send().unwrap();  
    // Grab Table Element Out of HTML
    let mut doc = Document::from_read(res).unwrap();

    let mut ProxyList = Vec::new();
    
    // Pass through each row in the table
    for node in doc.find(Attr("id", "proxylisttable").descendant(Name("tr"))) {
        let mut nodes = node.find(Name("td"));
        let ip = match nodes.next() {
            None => continue,
            Some(data) => {
                data.text()
            },
        };
        let port = match nodes.next() {
            None => continue,
            Some(data) => {
                data.text()
            },
        };
        let proxyUrl = format!("http://{}:{}", ip, port);
        ProxyList.push(proxyUrl.to_string());
    }
    return ProxyList;
}
/*
fn scrape(val: &str) -> Result<Vec<HashMap<String, String>>, String> {
    let mut vec = Vec::new();
    let mut book_reviews = HashMap::new();

    // Review some books.
    book_reviews.insert(
        "ABCD".to_string(),
        val.to_string(),
    );
    book_reviews.insert(
        "Grimms' Fairy Tales".to_string(),
        "Masterpiece.".to_string(),
    );
    book_reviews.insert(
        "Pride and Prejudice".to_string(),
        "Very enjoyable.".to_string(),
    );
    book_reviews.insert(
        "The Adventures of Sherlock Holmes".to_string(),
        "Eye lyked it alot.".to_string(),
    );
    vec.push(book_reviews.clone());
    vec.push(book_reviews);

    Ok(vec)
}*/
