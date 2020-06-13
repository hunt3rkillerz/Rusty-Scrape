# Rusty-Scrape
An OSINT tool designed to help pentesters audit external assets by performing password sprays and phising. The tool will collect a list of users with first names, last names and job titles and output them to a CSV file. This data can then be used to perform password sprays and/or compile a list of emails for phishing. The list is designed to have a large number of false positives. We view having a higher amount of false positives and therby a lower number of false negatives as an advantage, as emails sent to accounts which don't exist bounce and you cannot lockout accounts which do not exist on a password spray/
## Setup
The following command will build the application. Please note Rust and Cargo must first be installed.
```cargo build ```
## Usage
The intended method of using the tool is as follows.
```./rust_scrape -p <Company_Name>```
This is the standard method as it performs scraping via proxy servers and as such reduces the chance of being IP banned during a scan.
