# SSQL Examples

Each subdirectory here is a self-contained example project: a small schema,
some sample data, and one or more SSQL queries over it.

## What a project directory contains

- `schema.सं` — the table definitions, written in native SSQL.
- `seed.सं` (optional) — sample rows, also in native SSQL.
- Queries — `queries/*.सं` in the numbered projects; some of the named
  projects keep them in another subdirectory (e.g. `SAMSQL/*.सं`) or in the
  project root.
- `PROJECT.md` — the scenario, the data and its provenance, the SQL/SSQL
  concepts demonstrated, and the objectives. Many projects also have a
  `GUIDE.md` walkthrough.

`.sql` files next to a `.सं` file are compiler output, kept for reference.

## Projects

| Directory | Project | Domain |
|---|---|---|
| [`01_maven_toys_sales`](01_maven_toys_sales/PROJECT.md) | Maven Toys Sales | Retail |
| [`02_global_electronics`](02_global_electronics/PROJECT.md) | Global Electronics | Retail |
| [`03_us_candy_distributor`](03_us_candy_distributor/PROJECT.md) | US Candy Distributor | Supply Chain |
| [`04_customer_acquisition`](04_customer_acquisition/PROJECT.md) | Customer Acquisition | CRM |
| [`05_online_retail_returns`](05_online_retail_returns/PROJECT.md) | Online Retail Returns | Retail |
| [`06_pizza_throughput`](06_pizza_throughput/PROJECT.md) | Pizza Throughput | Food |
| [`07_restaurant_orders`](07_restaurant_orders/PROJECT.md) | Restaurant Orders | Food |
| [`08_ingredient_costs`](08_ingredient_costs/PROJECT.md) | Ingredient Costs | Food |
| [`09_startup_funding`](09_startup_funding/PROJECT.md) | Startup Funding | Finance |
| [`10_mavenflix_churn`](10_mavenflix_churn/PROJECT.md) | MavenFlix Churn | SaaS |
| [`11_sales_pipeline`](11_sales_pipeline/PROJECT.md) | B2B Sales Pipeline | CRM |
| [`12_unicorn_valuation`](12_unicorn_valuation/PROJECT.md) | Unicorn Valuation | Venture Capital |
| [`13_portfolio_exits`](13_portfolio_exits/PROJECT.md) | Portfolio Exits | Finance |
| [`14_marketing_roi`](14_marketing_roi/PROJECT.md) | Marketing ROI | Marketing |
| [`15_social_engagement`](15_social_engagement/PROJECT.md) | Social Engagement | Social Media |
| [`16_influencer_moving_avg`](16_influencer_moving_avg/PROJECT.md) | Influencer Moving Average | Marketing |
| [`17_funnel_drop_off`](17_funnel_drop_off/PROJECT.md) | Funnel Drop-off | E-Commerce |
| [`18_patient_readmission`](18_patient_readmission/PROJECT.md) | Patient Readmission Analysis | Healthcare |
| [`19_hr_org_chart`](19_hr_org_chart/PROJECT.md) | HR Org Chart | HR |
| [`20_safety_incident_logs`](20_safety_incident_logs/PROJECT.md) | Safety Incident Logs | Industrial Safety |
| [`21_flight_delay_risk`](21_flight_delay_risk/PROJECT.md) | Flight Delay Risk | Aviation |
| [`22_severe_weather_tracking`](22_severe_weather_tracking/PROJECT.md) | Severe Weather Tracking | Meteorology |
| [`23_athlete_performance_index`](23_athlete_performance_index/PROJECT.md) | Athlete Performance Index | Sports Analytics |
| [`24_hotel_booking_cancellations`](24_hotel_booking_cancellations/PROJECT.md) | Hotel Booking Cancellations | Hospitality |
| [`25_genomic_variant_frequencies`](25_genomic_variant_frequencies/PROJECT.md) | Genomic Variant Frequencies | Genomics / Bioinformatics |
| [`26_customer_cohorts`](26_customer_cohorts/PROJECT.md) | Customer Cohorts | SaaS / Customer Analytics |
| [`27_movie_ratings`](27_movie_ratings/PROJECT.md) | Movie Ratings & Reviews | Media & Entertainment |
| [`28_tech_ticket_aging`](28_tech_ticket_aging/PROJECT.md) | Tech Support Ticket Aging | Cloud & Tech Operations |
| [`29_cart_abandonment`](29_cart_abandonment/PROJECT.md) | Cart Abandonment Analysis | Retail / E-Commerce |
| [`30_server_resource_utilization`](30_server_resource_utilization/PROJECT.md) | Server Resource Utilization | DevOps / Cloud Infrastructure |
| [`coffee_sales`](coffee_sales/PROJECT.md) | Coffee Sales | Food & Beverage |
| [`ecommerce`](ecommerce/PROJECT.md) | Ecommerce Monthly Insights | Finance |
| [`gurukula_management`](gurukula_management/PROJECT.md) | Gurukula Management — SQL Feature Verification Suite | Education (traditional gurukula/school) |
| [`pizza_sales`](pizza_sales/PROJECT.md) | Pizza Sales | Food & Beverage |
| [`restaurant_orders`](restaurant_orders/PROJECT.md) | Restaurant Orders | Food & Beverage |
| [`simple`](simple/PROJECT.md) | Simple Basic SELECT | Education |
| [`university_bookstore`](university_bookstore/PROJECT.md) | University Book Store — SSQL Verification Corpus | Retail/bookstore |

## Compiling a query

```bash
cargo run --bin ssql_compiler -- examples/01_maven_toys_sales/queries/q1_products.सं --dialect sqlite --output out.sql
```

`--dialect` is `sqlite` (the default) or `postgres`. Without `--output`, the
SQL is written next to the input file with a `.sql` extension. The schema is
picked up automatically from the `schema.सं` in the query's directory or a
parent directory.

## Verifying all examples

```bash
uv run verify_all.py --dialect sqlite
```

This compiles and runs every project's schema, seed, and queries
(`--dialect` also accepts `postgres` and `all`).
