# Pollinator 3000
A light, fast and user-friendly server application for polls, surveys, ballots 
etc.

Written in Rust, HTML/CSS and JavaScript.
Uses Sqlite3 for data storage.

A testing instance of this application is hosted [here](http://84.10.40.254/ "Testing instance")

![Ranked poll results](resources/results.png)

Goals:
 - Not requiring users to create accounts, confirm emails etc.
 - Self-hostable with minimal effort
 - Understandable for inexperienced users
 - Able to create more advanced types of polls
 - Powerful REST API for scripts and automation
 - Good documentation
 - Accessible webpage design

![Simple voting](resources/simple.png)
![Ranked voting](resources/ranked.png)

## Usage
### Creating the database
In the db/ folder or anywhere the server has access to, enter

```sqlite3 main.db < db.sql```

This will create a database called `main.db` from a template stored in `db.
sql`.

### Running the server
Simply run the server (`cargo run` or run the compiled binary).
By default, it will look for a database in `db/main.db`.

For help on commandline arguments, run the server with a `help` argument:
```
USAGE: poll (DATABASE_PATH) (BIND_ADDRESS)
The default DATABASE_PATH is ./db/main.db
The default BIND_ADDRESS is 0.0.0.0:8080.
```
### Environmental variables
Some functionality of the server can be altered by setting specific 
environmental variables:
 - `POLL_ADMIN_TOKEN` - A "password" for the website administrator. When 
   set, it enables the administration webpage `{website}/admin`.
 - `POLL_CREATE_LIMIT` - The amount of time (in seconds) that a single IP 
   address has to wait between poll creation requests.
 - `POLL_VOTE_LIMIT` - The amount of time (in seconds) that a single IP
   address has to wait between voting requests on a single poll.
 - `POLL_CLEANUP_INTERVAL` - The amount of time (in seconds) between runs 
   of a thread responsible for cleaning up old IP limits (a sort of garbage 
   collector).

## REST API
For each endpoint's API arguments, see it's handler function's documentation.
### API Example
Type: `cargo doc --open`, navigate to the handler function (example: 
`handle_create_desc`) and read its doc comment:
```
Handles complete poll creation requests
Params:
 - name: Poll name (String)
 - type: PollType enum variant, see PollType::try_parse for parsing format
 - data: To be parsed as a PollFormat trait object, see the corresponding
   PollType::from_data function for the proper format
```
Then, send an HTTP POST request to that endpoint with your data.
