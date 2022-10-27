# Olx Scrapie

olx.ro (and conex storia.ro items) real estate scraper ().

## Todo
- [ ] extract data from OLX pages
  - [x] async workers
  - [x] establish data structure
  - ~[ ] selectors and processors~
  - [x] use embedded ad JSON

- [ ] Sessions management (upgrade listing sessions)
  - [ ] List sessions
  - [ ] Deleting a session
  - [ ] Show session stats

- [x] implement session persistence
  - [x] save current session in database
  - [x] move each job in a queue

- [x] scrape item page
  - [x] olx
  - [x] storia
