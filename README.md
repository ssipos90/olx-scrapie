# Olx Scrapie

olx.ro (and connex storia.ro items) real estate scraper ().

## Todo
- [ ] extract data from pages
  - [x] async workers
  - [x] establish data structure
  - [ ] selectors and processors

- [ ] Sessions management (upgrade listing sessions)
  - [ ] List sessions
  - [ ] Deleting a session
  - [ ] Show session stats

- [x] implement session persistance
  - [x] save current session in database
  - [x] move each job in a queue

- [x] scrape item page
  - [x] olx
  - [x] storia
