# Project 01: Maven Toys Sales - Beginner's Guide

This project introduces basic SSQL query concepts like table creation, simple selections, joining tables, and calculating metrics with aggregation.

---

## 🎯 Deliverables & Objectives

1. **Objective 1**: Retrieve all products with their name, cost, and price.
2. **Objective 2**: Calculate total revenue per store.

---

## 🛠️ Step-by-Step Solution

### 1. DDL Schema Definition (`schema.सं`)
SSQL defines schemas natively. We define three tables: `क्रीडनकभाण्डाराः` (stores), `उत्पादाः` (products), and `विक्रयाः` (sales). Note that column constraints are placed at the **beginning** of the definition:
```ssql
क्रीडनकभाण्डाराः इति पटलम् "stores" इति आख्या क्रीडनकभाण्डार इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "store_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    अनिवार्यम् नाम इति वाक्यम् "store_name" इति आख्या नामन् इति नपुंसकलिङ्गम् एकवचनम् च
    नगरम् इति वाक्यम् "store_city" इति आख्या नगर इति नपुंसकलिङ्गम् एकवचनम् च
    स्थलम् इति वाक्यम् "store_location" इति आख्या स्थल इति नपुंसकलिङ्गम् एकवचनम् च
    प्रारम्भदिनाङ्कः इति दिनाङ्कः "store_open_date" इति आख्या प्रारम्भदिनाङ्क इति पुंलिङ्गम् एकवचनम् च
  इति पटलम् ।

उत्पादाः इति पटलम् "products" इति आख्या उत्पाद इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "product_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    अनिवार्यम् नाम इति वाक्यम् "product_name" इति आख्या नामन् इति नपुंसकलिङ्गम् एकवचनम् च
    वर्गः इति वाक्यम् "product_category" इति आख्या वर्ग इति पुंलिङ्गम् एकवचनम् च
    व्ययः इति वास्तविकः "product_cost" इति आख्या व्यय इति पुंलिङ्गम् एकवचनम् च
    मूल्यम् इति वास्तविकः "product_price" इति आख्या मूल्य इति नपुंसकलिङ्गम् एकवचनम् च
  इति पटलम् ।

विक्रयाः इति पटलम् "sales" इति आख्या विक्रय इति पुंलिङ्गम् बहुवचनम् रच्यताम् यत्र
    क्रमिका प्राथमिकी सङ्ख्या इति पूर्णाङ्कः "sale_id" इति आख्या सङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    विक्रयदिनाङ्कः इति दिनाङ्कः "date" इति आख्या विक्रयदिनाङ्क इति पुंलिङ्गम् एकवचनम् च
     क्रीडनकभाण्डारेषु बद्धा  अनिवार्या भाण्डारसङ्ख्या इति पूर्णाङ्कः "store_id" इति आख्या भाण्डारसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
     उत्पादेषु बद्धा  अनिवार्या उत्पादसङ्ख्या इति पूर्णाङ्कः "product_id" इति आख्या उत्पादसङ्ख्या इति स्त्रीलिङ्गम् एकवचनम् च
    अनिवार्या मात्रा इति पूर्णाङ्कः "units" इति आख्या मात्रा इति स्त्रीलिङ्गम् एकवचनम् च
  इति पटलम् ।
```

### 2. Solving Objective 1: Retrieve Product List
We want to select `name`, `product_cost`, and `product_price` from the `products` table. 
* **Sanskritization**:
  * We select from `उत्पादाः` (masculine plural). In SSQL, the source table must be in the Ablative case (`Panchami Vibhakti`), which is `उत्पादेभ्यः`.
  * The fields we select are `नाम` (name), `व्ययः` (cost), and `मूल्यम्` (price). We join them with `च` (and).
  * Since we select three columns, we use the plural fetch verb `आह्रियन्ताम्`.
* **Query** (`queries/q1_products.सं`):
  ```sanskrit
  उत्पादेभ्यः नाम च व्ययः च मूल्यम् च आह्रियन्ताम् ।
  ```

### 3. Solving Objective 2: Calculate Store Revenue
We calculate revenue as the sum of `units * product_price` for each store.
* **Sanskritization**:
  * We query from `विक्रयाः` (sales), inflected as Ablative `विक्रयेभ्यः`.
  * We perform a `JOIN` with the `stores` table using `क्रीडनकभाण्डारैः सह ... इति मेलनेन`.
  * We join the `products` table using `उत्पादैः सह ... इति मेलनेन`.
  * We group by the store name using `क्रीडनकभाण्डारस्य नाम इति समूहेन`.
  * The selected fields are the store name (`क्रीडनकभाण्डारस्य नाम`) and the calculated revenue.
  * In SSQL, math operations use postfix notation: `units * price` becomes `विक्रयस्य मात्रा च उत्पादस्य मूल्यम् च इत्यनयोः गुणनम्`. We wrap this inside a sum aggregate `इत्यस्य योगः राजस्वम् इति आख्या`.
  * Since we select exactly 2 columns, we use the dual fetch verb `आह्रियेताम्`.
* **Query** (`queries/q2_revenue_per_store.सं`):
  ```sanskrit
  विक्रयेभ्यः
      क्रीडनकभाण्डारैः सह विक्रयस्य भाण्डारसङ्ख्या क्रीडनकभाण्डारस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
      उत्पादैः सह विक्रयस्य उत्पादसङ्ख्या उत्पादस्य सङ्ख्या इत्यनया समा अस्ति इति मेलनेन
      क्रीडनकभाण्डारस्य नाम इति समूहेन
      विक्रयस्य मात्रा च उत्पादस्य मूल्यम् च इत्यनयोः गुणनम् इत्यस्य योगः राजस्वम् इति आख्या च
      क्रीडनकभाण्डारस्य नाम च
      आह्रियेताम् ।
  ```

---

## 🧪 Local Verification

You can verify that these queries compile and execute correctly by running the `verify_all.py` script from the repository root:
```bash
uv run python3 verify_all.py
```
This compiles the queries, executes them against both SQLite and PostgreSQL, and compares the rows with the reviewed results in `tests/golden/results/`. Pass `--dialect sqlite` to skip PostgreSQL; `--compile-only` checks compilation alone and executes nothing.
