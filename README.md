Rust Multi-DB Microservice (Axum + SeaQuery + AWS)
A high-performance, scalable backend microservice built with Rust and Axum. This project demonstrates a Polyglot Persistence architecture, where multiple database technologies are used in tandem to handle different data requirements (Relational, Document, Key-Value, and Event-Driven).

🏗️ Architecture & Logic
The core logic of this service follows the "Write-Through" and "Event-Sourcing" patterns. When a single Task is created, the system synchronizes data across six different environments:

MySQL (Primary Source of Truth): Uses SeaQuery (dynamic query builder) to ensure strictly typed relational data storage.

AWS DynamoDB (Global JSON Storage): Stores the same task as a flexible JSON document for high-scale read availability.

MongoDB (Audit Trail): Every "Create" operation is logged as a document to maintain a historical audit trail.

Redis (State Cache): Tracks the "Last Operation" performed on the API for instant state monitoring.

AWS SQS (Decoupling): Pushes the task into a message queue to be processed by background workers (asynchronous processing).

AWS SNS (Broadcasting): Publishes a notification to a topic, allowing other microservices to subscribe to "Task Created" events.

🛠️ Technology Stack
Component	Technology	Purpose
Framework	Axum	High-performance, async web framework.
SQL ORM	SeaQuery + SQLx	Type-safe, dynamic SQL generation for MySQL.
NoSQL (Doc)	MongoDB	Audit logging and flexible schema storage.
NoSQL (KV)	Redis	High-speed operational caching.
Cloud (NoSQL)	AWS DynamoDB	Managed JSON document storage.
Messaging	AWS SQS	Asynchronous task queuing.
Pub/Sub	AWS SNS	Real-time event broadcasting.
🚀 Getting Started
1. Prerequisites

Rustc 1.91.1+ (Required for the latest AWS SDK).

Docker (Recommended for local MySQL/Mongo/Redis).

AWS IAM User with DynamoDB, SQS, and SNS permissions.

2. Environment Setup

Create a .env file in the root directory:

Code snippet
PORT=3001
DATABASE_URL=mysql://user:pass@localhost:3306/db_name
MONGODB_URI=mongodb://localhost:27017
REDIS_URL=redis://127.0.0.1:6379
AWS_REGION=ap-south-1
AWS_ACCESS_KEY_ID=your_key
AWS_SECRET_ACCESS_KEY=your_secret
SQS_QUEUE_URL=
SNS_TOPIC_ARN=
3. Database Initialization

The application automatically handles MySQL table creation on startup using SeaQuery. For DynamoDB, ensure a table named Tasks exists with id (Number) as the Partition Key.

4. Running the App

Bash
cargo run
📡 API Endpoints
Method	Endpoint	Description
GET	/tasks	Fetches all tasks from MySQL.
POST	/tasks	Creates a task and syncs to MySQL, Dynamo, Mongo, Redis, SQS, and SNS.
PUT	/tasks/{id}	Updates a task in MySQL.
DELETE	/tasks/{id}	Deletes a task from MySQL.
🛡️ Security Note
Never commit your .env file. This project uses GitHub Secret Scanning. If you accidentally commit AWS keys, rotate them immediately in the IAM Console and use git reset to scrub your local history.

🧠 Core Logic Flow (The create_task Handler)
Validation: Receive and deserialize JSON into a Task struct.

Persistence: Build and execute a MySQL INSERT via SeaQuery.

Cloud Sync: Map the Rust struct to a DynamoDB Item using serde_dynamo.

Queueing: Stringify the task and push to SQS for downstream consumers.

Messaging: Publish a "New Task" alert to an SNS Topic.

Logging: Update Redis with the operation type and save an audit doc in MongoDB.

Developed with ❤️ in Rust. Author: Saurabh Kamble
