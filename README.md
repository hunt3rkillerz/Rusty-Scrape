# Rusty-Scrape
An OSINT tool designed to help pentesters audit external assets by performing password sprays and phishing. The tool will collect a list of users with first names, last names and job titles and output them to a CSV file. This data can then be used to perform password sprays and/or compile a list of emails for phishing. The list is designed to have a large number of false positives. We view having a higher amount of false positives and thereby a lower number of false negatives as an advantage, as emails sent to accounts which don't exist bounce and you cannot lockout accounts which do not exist on a password spray.

## How Does It Work
The tool will generate a large number of Bing searches with traditional "Google Dorks" queries attempting to locate results from LinkedIn. The tool allows you to send all requests via a proxy/proxies scraped from https://www.sslproxies.org/. Please note all requests are performed using HTTP for speed and consistency.

## Setup
The following command will build the application. Please note Rust and Cargo must first be installed.

```cargo build ```

## Quick Start
The intended method of using the tool is as follows.

```./rust_scrape -p <Company_Name>```

This is the standard method as it performs scraping via proxy servers and as such reduces the chance of being IP banned during a scan.

## Usage
Usage of the tool follows the following format:

`./rust_scrape [-p -o <Output_File> -w <Word_List>] <Company_Name>`

### Flags
`-p, --proxy`      Use this flag to scan via proxies. If set to true the scan WILL take much longer.

`-o, --output <Output>`        Name of the file to output to (CSV FORMAT).

`-w, --wordlist <WordList>`    Path to a wordlist file which can be used for the scan.
