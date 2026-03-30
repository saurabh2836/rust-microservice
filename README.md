# 🦀 Rust Multi-DB Microservice (Axum + SeaQuery + AWS)

A high-performance, production-ready backend microservice built with **Rust**. This project demonstrates a **Polyglot Persistence** architecture, where six different database and cloud technologies are synchronized in real-time to handle relational data, document storage, caching, and event-driven messaging.

---

## 🏗️ Architecture & Business Logic

This service follows the **Write-Through** and **Event-Sourcing** patterns. When a `Task` is created via the API, the system ensures data integrity and availability across the following stack:

1.  **MySQL (Primary Store):** Uses **SeaQuery** (dynamic query builder) and **SQLx** for strictly typed relational storage.
2.  **AWS DynamoDB (Global JSON):** Stores the task as a flexible JSON document for high-scale NoSQL availability.
3.  **MongoDB (Audit Trail):** Every "Create" operation is logged as a document to maintain a historical audit trail.
4.  **Redis (State Cache):** Tracks the "Last Operation" performed for instant system state monitoring.
5.  **AWS SQS (Decoupling):** Pushes the task into a message queue for asynchronous processing by background workers.
6.  **AWS SNS (Broadcasting):** Publishes a real-time notification to a topic for cross-service event orchestration.

---

## 🛠️ Technology Stack

| Component | Technology | Purpose |
| :--- | :--- | :--- |
| **Framework** | Axum | High-performance, async web framework. |
| **SQL Engine** | SeaQuery + SQLx | Type-safe, dynamic SQL generation for MySQL. |
| **Document DB** | MongoDB | Flexible schema for audit logging. |
| **Cache** | Redis | High-speed operational key-value store. |
| **Cloud NoSQL** | AWS DynamoDB | Managed JSON document storage. |
| **Queueing** | AWS SQS | Asynchronous task decoupling. |
| **Pub/Sub** | AWS SNS | Real-time event broadcasting. |

---

## 🚀 Getting Started

### 1. Prerequisites
* **Rustc 1.91.1+** (Required for the latest AWS SDK).
* **MySQL, MongoDB, and Redis** (Running locally or via Docker).
* **AWS IAM User** with `DynamoDB`, `SQS`, and `SNS` full access permissions.

### 2. Environment Setup
Create a `.env` file in the root directory (This file is ignored by Git for security):

```env
PORT=3001
DATABASE_URL=mysql://user:password@localhost:3306/your_db
MONGODB_URI=mongodb://localhost:27017
REDIS_URL=redis://127.0.0.1:6379

# AWS Configuration
AWS_REGION=ap-south-1
AWS_ACCESS_KEY_ID=YOUR_ACCESS_KEY
AWS_SECRET_ACCESS_KEY=YOUR_SECRET_KEY
SQS_QUEUE_URL=[)
SNS_TOPIC_ARN=
