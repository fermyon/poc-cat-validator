# Common Access Token Validator [POC]

This repository contains a simple PoC to demonstrate common access token (CAT) validation. Although this implementation validates certain claims according to the CAT Specification (CAT-5007-B), it's not implementing all aspects of the specification. 

**This implementation is not meant to be used in a production environment.**

## Custom additions

The validator defines a set of block lists, which could be used to invalidate tokens based on individual requirements. Custom blocklist are checked once token integrity has been confirmed and the "default" token validation has passed. 

The following block lists are supported:

- Country
- User-Agent
- Subject
- CIDR

API endpoints for managing block lists are not protected, as we considered authentication and authorization out of scope for this PoC.

## Supported Claims 

CAT validation checks the following claims:

- `iss`
- `aud` 
- `exp`
- `nbf`
- `catu`
- `catm`
- `cath`
- `catv`
- `catgetoiso3166`
- `catnip` (Excluding ASN)


## Running a perf test

Need to have k6 installed.

```bash
export TOKEN=<cat token>
export SIMPLE_URL=<simple validation url>
export KV_URL=<kv validation url>

cd perf-test
npm install
node run_and_report.js
```