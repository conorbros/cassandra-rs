#[macro_use(stmt)]
extern crate cassandra_cpp;
extern crate slog;
extern crate futures;

mod help;

use cassandra_cpp::*;

use std::sync::Arc;
use std::sync::Mutex;
use slog::*;
use futures::Future;
use std::time::Duration;
use std::thread;
use std::thread::sleep;

/// Simple drain which accumulates all messages written to it.
#[derive(Clone)]
struct MyDrain(Arc<Mutex<String>>);

impl Default for MyDrain {
    fn default() -> Self {
        MyDrain(Arc::new(Mutex::new("".to_string())))
    }
}

impl Drain for MyDrain {
    type Ok = ();
    type Err = ();

    fn log(&self, record: &Record, _values: &OwnedKVList) -> ::std::result::Result<Self::Ok, Self::Err> {
        self.0.lock().unwrap().push_str(&format!("{}", record.msg()));
        Ok(())
    }
}

#[test]
fn test_logging() {
    let drain = MyDrain::default();
    let logger = Logger::root(drain.clone().fuse(), o!());

    set_level(LogLevel::WARN);
    set_logger(Some(logger));

    let mut cluster = Cluster::default();
    cluster.set_contact_points("absolute-gibberish.invalid").unwrap();
    cluster.connect().expect_err("Should fail to connect");

    let log_output: String = drain.0.lock().unwrap().clone();
    assert!(log_output.contains("Unable to resolve address for absolute-gibberish.invalid"), log_output);
}

#[test]
fn test_metrics() {

    let query = stmt!("CREATE KEYSPACE IF NOT EXISTS cycling WITH REPLICATION = {
   'class' : 'SimpleStrategy',
   'replication_factor' : 1
  };");

    let query2 = stmt!("CREATE TABLE IF NOT EXISTS cycling.cyclist_name (
   id UUID PRIMARY KEY,
   lastname text,
   firstname text );"); //create table

    let query3 = stmt!("INSERT INTO cycling.cyclist_name (id, lastname, firstname)
  VALUES (6ab09bec-e68e-48d9-a5f8-97e6fb4c9b47, 'KRUIKSWIJK','Steven')
  USING TTL 86400 AND TIMESTAMP 123456789;");

    let session = help::create_test_session();
    session.execute(&query).wait().unwrap();
    session.execute(&query2).wait().unwrap();
    session.execute(&query3).wait().unwrap();

    let metrics = session.get_metrics();

    assert_eq!(metrics.total_connections, 1);
    assert!(metrics.min_us > 0);
}
