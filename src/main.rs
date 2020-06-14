#![allow(non_snake_case)]
extern crate reqwest;
extern crate select;
extern crate rand;

use std::vec::Vec;
use std::env;
use std::fs::{self, File};
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
    let matches = App::new("Rusty Scrape")
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
        .arg(Arg::with_name("output")
            .short('o')
            .long("output")
            .value_name("Output")
            .about("Name of the file to output to (CSV FORMAT).")
            .takes_value(true))
        .get_matches();

    let res = splitVector(matches.is_present("proxy"), matches.value_of("CompanyName").unwrap(), getWordList(matches.value_of("wordlist").unwrap_or("wordlists/default.csv")));
    
    let mut outputFile = "output.csv";
    if matches.is_present("output") {
        outputFile = match matches.value_of("output") {
            Some(val) => val,
            None => outputFile
        };
    }
    outputToFile(res, outputFile);

}

// Prints results to file
fn outputToFile(data: Vec<Vec<String>>, fileName: &str) -> () {
    let mut fileData: String = "First Name,Last Name,Job Title\n".to_owned();

    for user in data {
        let temp = format!("{},{},{}\n", user[0], user[1], user[2]);
        fileData.push_str(&temp);
    }

    fs::write(fileName, fileData).expect("Unable to write file");
}

// Reads the profession word list out of a file
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

// Utility to help process the Bing Data
fn processBingData(doc: Document) -> Vec<Vec<String>> {
    let mut userVec = Vec::new();
    for node in doc.find(Name("li")) {
        let part1 = match node.find(Name("h2")).next() {
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

        let tokens: Vec<&str> = relData.split("-").collect();
        if tokens.len() < 3 {
            // Wrong type of result
            continue
        }
        
        // Remove trailing space
        let mut nameData = tokens[0].to_string();
        nameData.pop();
        
        // Bad result case 1
        if tokens.len() < 2 {
            continue;
        }
        // Bad result case 2
        if tokens[1].len() < 2 {
            continue;
        }
        
        // Remove trailing & leading space
        let job = &tokens[1][1..tokens[1].len()-1];

        let nameTokens: Vec<&str> = nameData.split(" ").collect();
        
        if nameTokens.len() < 2 {
            continue
        }

        userVec.push(vec![nameTokens[0].to_string(), nameTokens[1].to_string(), job.to_string()]);
    }

    return userVec;
}

// Scrapes the data from the Bing Search
fn scrape(prof: &str, company_name: &str, proxy: Option<Vec<String>>) -> Vec<Vec<String>> {
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
        let client;
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
                // 20 Second Timeout because this is supposed to work
                Duration::new(20, 0)
            ).build().unwrap();
        }
        else {
            client = reqwest::blocking::Client::builder().
            timeout(
                // 20 Second Timeout because this is supposed to work
                Duration::new(20, 0)
            ).build().unwrap();
        }

        let res = match client.get(&searchURL)
                .header(USER_AGENT, getRandomUserAgent())
                .send(){
                    Err(_e) => {
                        continue
                    },
                    Ok(res) => res
        };  

        let doc = match Document::from_read(res) {
            Err(_e) => continue,
            Ok(doc) => doc
        };
        if doc.find(Name("li")).count() < 8 && isProxy {
            println!("BLOCKER");
            continue
        }
        return processBingData(doc);
    }
}

// Splits the vector of profession lists and then performs searches for users in a multi-threaded manner
fn splitVector(useProxy: bool, company_name: &str, professionList: Vec<String>) -> Vec<Vec<String>> {
    let proxy_list = fetchProxyList();
    let data: Vec<Vec<Vec<String>>>;
    if useProxy {
        data = professionList.par_iter()
            .map(|i| scrape(&i, &company_name.clone(), Some(proxy_list.clone())))
            .collect();
    }
    else {
        data = professionList.par_iter() 
            .map(|i| scrape(&i, &company_name.clone(), None))
            .collect();
    }
    let mut cleanArr = Vec::new();
    for arr in data {
        for elem in arr {
            cleanArr.push(elem);
        }
    }
    return cleanArr;
}


// Finds a proxy that works and returns the URL
fn findProxy(proxy_list: &mut Vec<String>) -> String {
    let mut rng = thread_rng();
    while proxy_list.len() > 0 {
        // Select a proxy from the list at random and pop it
        let randVal = rng.gen_range(0, proxy_list.len()-1);
        let proxy = proxy_list[randVal].clone();
        proxy_list.remove(randVal);
        const URL: &str = "http://www.bing.com/search?q=Test+Search&qs=n&form=QBRE&sp=-1&ghc=1&pq=test+searc&sc=5-10&sk=&cvid=B580BE8B2CDB4AB7817D09B5011F1A6C";
        let client = reqwest::blocking::Client::builder().proxy(
            reqwest::Proxy::all(&proxy).unwrap()
        ).timeout(
            // 10 Second Timeout means basically everything works
            Duration::new(10, 0)
        ).build().unwrap();
        match client.get(URL)
            .header(USER_AGENT, getRandomUserAgent())
            .send()
        {
            Err(_e) => {
                continue
            },
            Ok(res) => res
        };  

        // We found our proxy
        return proxy.to_string();
    }
    // Only here because this method is disgusting
    return "".to_string();
}

// Returns a pseudo random user agent from a hardcoded list
fn getRandomUserAgent() -> String {
    // Hardcoding is the way of the future
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
    // Random Selection
    user_agent_list.choose(&mut rand::thread_rng()).unwrap().to_string()
}

// Fetches a list of potential Proxy URL's in a vector.
fn fetchProxyList() -> Vec<String> {
    // This web page contains a table of proxy servers
    const URL: &str = "https://www.sslproxies.org/";
    let client = reqwest::blocking::Client::new();

    // Requesting the page
    let res = client.get(URL)
        .header(USER_AGENT, getRandomUserAgent())
        .send().unwrap();  
    // Grab Table Element Out of HTML
    let doc = Document::from_read(res).unwrap();

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
