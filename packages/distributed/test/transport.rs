use streamweave_distributed::{Message, MessageType};
use streamweave_distributed::{TcpStreamTransport, TransportError, TransportMessage};
use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

#[test]
fn test_tcp_stream_transport_new() {
  let transport = TcpStreamTransport::new();
  assert_eq!(transport.sequence, 0);
}

#[test]
fn test_tcp_stream_transport_default() {
  let transport = TcpStreamTransport::default();
  assert_eq!(transport.sequence, 0);
}

#[test]
fn test_tcp_stream_transport_next_sequence() {
  let mut transport = TcpStreamTransport::new();
  assert_eq!(transport.next_sequence(), 1);
  assert_eq!(transport.next_sequence(), 2);
  assert_eq!(transport.next_sequence(), 3);
  assert_eq!(transport.sequence, 3);
}

#[tokio::test]
async fn test_send_message_success() {
  let message = Message::new(
    MessageType::Heartbeat,
    b"test payload".to_vec(),
    "source".to_string(),
    "destination".to_string(),
  );

  let (mut writer, mut reader) = duplex(1024);

  let result = TcpStreamTransport::send_message(&mut writer, &message).await;
  assert!(result.is_ok());

  // Verify the data was written correctly
  let length = reader.read_u32().await.unwrap();
  let mut buffer = vec![0u8; length as usize];
  reader.read_exact(&mut buffer).await.unwrap();
  let received: Message = serde_json::from_slice(&buffer).unwrap();
  assert_eq!(received.message_type, message.message_type);
  assert_eq!(received.payload, message.payload);
}

#[tokio::test]
async fn test_receive_message_success() {
  let original_message = Message::new(
    MessageType::TaskAssignment,
    b"task data".to_vec(),
    "coordinator".to_string(),
    "worker1".to_string(),
  );

  let (mut writer, mut reader) = duplex(1024);

  // Write the message first
  TcpStreamTransport::send_message(&mut writer, &original_message)
    .await
    .unwrap();

  // Now receive it
  let result = TcpStreamTransport::receive_message(&mut reader).await;
  assert!(result.is_ok());
  let received = result.unwrap();
  assert_eq!(received.message_type, original_message.message_type);
  assert_eq!(received.payload, original_message.payload);
  assert_eq!(received.source, original_message.source);
  assert_eq!(received.destination, original_message.destination);
}

#[tokio::test]
async fn test_receive_message_serialization_error() {
  let (mut writer, mut reader) = duplex(1024);

  // Write invalid JSON data
  let invalid_data = b"not valid json";
  writer.write_u32(invalid_data.len() as u32).await.unwrap();
  writer.write_all(invalid_data).await.unwrap();
  writer.flush().await.unwrap();

  let result = TcpStreamTransport::receive_message(&mut reader).await;
  assert!(result.is_err());
  match result.unwrap_err() {
    TransportError::Serialization(_) => {}
    _ => panic!("Expected Serialization error"),
  }
}

#[test]
fn test_transport_message_new() {
  let msg = TransportMessage {
    payload: 42,
    sequence: 1,
    metadata: Some(serde_json::json!({"key": "value"})),
  };

  assert_eq!(msg.payload, 42);
  assert_eq!(msg.sequence, 1);
  assert_eq!(msg.metadata, Some(serde_json::json!({"key": "value"})));
}

#[test]
fn test_transport_message_clone() {
  let msg1 = TransportMessage {
    payload: "test".to_string(),
    sequence: 5,
    metadata: None,
  };
  let msg2 = msg1.clone();

  assert_eq!(msg1.payload, msg2.payload);
  assert_eq!(msg1.sequence, msg2.sequence);
  assert_eq!(msg1.metadata, msg2.metadata);
}

#[test]
fn test_transport_message_debug() {
  let msg = TransportMessage {
    payload: 100,
    sequence: 10,
    metadata: None,
  };

  let debug_str = format!("{:?}", msg);
  assert!(!debug_str.is_empty());
  assert!(debug_str.contains("100"));
  assert!(debug_str.contains("10"));
}

#[test]
fn test_transport_message_serialize_deserialize() {
  let original = TransportMessage {
    payload: vec![1, 2, 3, 4, 5],
    sequence: 42,
    metadata: Some(serde_json::json!({"test": true})),
  };

  let serialized = serde_json::to_string(&original).unwrap();
  let deserialized: TransportMessage<Vec<u8>> = serde_json::from_str(&serialized).unwrap();

  assert_eq!(original.payload, deserialized.payload);
  assert_eq!(original.sequence, deserialized.sequence);
  assert_eq!(original.metadata, deserialized.metadata);
}

#[test]
fn test_transport_error_io() {
  let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
  let transport_error = TransportError::from(io_error);

  match transport_error {
    TransportError::Io(e) => {
      assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
    }
    _ => panic!("Expected Io error"),
  }
}

#[test]
fn test_transport_error_serialization() {
  let error = TransportError::Serialization("invalid json".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Serialization error"));
  assert!(error_str.contains("invalid json"));
}

#[test]
fn test_transport_error_stream_ended() {
  let error = TransportError::StreamEnded;
  let error_str = format!("{}", error);
  assert!(error_str.contains("Stream ended unexpectedly"));
}

#[test]
fn test_transport_error_other() {
  let error = TransportError::Other("custom error".to_string());
  let error_str = format!("{}", error);
  assert!(error_str.contains("Transport error"));
  assert!(error_str.contains("custom error"));
}

#[test]
fn test_transport_error_debug() {
  let error = TransportError::StreamEnded;
  let debug_str = format!("{:?}", error);
  assert!(!debug_str.is_empty());
}

#[tokio::test]
async fn test_send_receive_roundtrip() {
  let original_message = Message::new(
    MessageType::TaskComplete,
    b"completion data".to_vec(),
    "worker1".to_string(),
    "coordinator".to_string(),
  )
  .with_correlation_id("corr-123".to_string());

  let (mut writer, mut reader) = duplex(1024);

  // Send message
  let send_result = TcpStreamTransport::send_message(&mut writer, &original_message).await;
  assert!(send_result.is_ok());

  // Receive message
  let receive_result = TcpStreamTransport::receive_message(&mut reader).await;
  assert!(receive_result.is_ok());
  let received = receive_result.unwrap();

  assert_eq!(received.message_type, original_message.message_type);
  assert_eq!(received.payload, original_message.payload);
  assert_eq!(received.source, original_message.source);
  assert_eq!(received.destination, original_message.destination);
  assert_eq!(received.correlation_id, original_message.correlation_id);
}

#[tokio::test]
async fn test_send_message_empty_payload() {
  let message = Message::new(
    MessageType::Heartbeat,
    vec![],
    "source".to_string(),
    "destination".to_string(),
  );

  let (mut writer, mut reader) = duplex(1024);

  let result = TcpStreamTransport::send_message(&mut writer, &message).await;
  assert!(result.is_ok());

  // Verify empty payload was sent correctly
  let length = reader.read_u32().await.unwrap();
  let mut buffer = vec![0u8; length as usize];
  reader.read_exact(&mut buffer).await.unwrap();
  let received: Message = serde_json::from_slice(&buffer).unwrap();
  assert_eq!(received.payload, Vec::<u8>::new());
}

#[tokio::test]
async fn test_receive_message_empty_payload() {
  let original_message = Message::new(
    MessageType::Heartbeat,
    vec![],
    "source".to_string(),
    "destination".to_string(),
  );

  let (mut writer, mut reader) = duplex(1024);

  TcpStreamTransport::send_message(&mut writer, &original_message)
    .await
    .unwrap();

  let result = TcpStreamTransport::receive_message(&mut reader).await;
  assert!(result.is_ok());
  let received = result.unwrap();
  assert_eq!(received.payload, Vec::<u8>::new());
}

#[test]
fn test_transport_message_with_metadata() {
  let msg = TransportMessage {
    payload: "test".to_string(),
    sequence: 1,
    metadata: Some(serde_json::json!({
      "timestamp": 1234567890,
      "node_id": "node-1"
    })),
  };

  assert_eq!(msg.metadata.as_ref().unwrap()["node_id"], "node-1");
}

#[test]
fn test_transport_message_without_metadata() {
  let msg = TransportMessage {
    payload: 42,
    sequence: 2,
    metadata: None,
  };

  assert_eq!(msg.metadata, None);
}

#[test]
fn test_transport_message_different_types() {
  let msg_i32 = TransportMessage {
    payload: 100,
    sequence: 1,
    metadata: None,
  };

  let msg_string = TransportMessage {
    payload: "hello".to_string(),
    sequence: 2,
    metadata: None,
  };

  let msg_vec = TransportMessage {
    payload: vec![1, 2, 3],
    sequence: 3,
    metadata: None,
  };

  assert_eq!(msg_i32.payload, 100);
  assert_eq!(msg_string.payload, "hello");
  assert_eq!(msg_vec.payload, vec![1, 2, 3]);
}
