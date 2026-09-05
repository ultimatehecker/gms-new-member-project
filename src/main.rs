use std::sync::mpsc::{Sender, channel};

#[derive(Debug, PartialEq)]
enum SampleData {
    Integer(i32),
    Float(f32),
    Boolean(bool),
}

#[derive(Debug, PartialEq)]
struct Sample {
    name: String,
    data: SampleData
}

/// Encodes a Sample into a Little Endian byte array
/// * `sample`: A sample to encode
/// * `buffer`: A buffer which is any amount bytes long, which is muttable to write the bytes to
fn encode(sample: &Sample, buffer: &mut [u8]) {
    let mut i = 0;

    buffer[i..i + 4].copy_from_slice(&0u32.to_le_bytes()); // Add the header of four bytes of zeros
    i += 4;

    let string_length = sample.name.len() as u32;
    buffer[i..i + 4].copy_from_slice(&string_length.to_le_bytes()); // Add the string length to the buffer
    i += 4;

    let name_bytes = sample.name.as_bytes();
    buffer[i..i + name_bytes.len()].copy_from_slice(name_bytes); // Add the string name to the buffer
    i += name_bytes.len();

    match sample.data { // Match between the different data types depending on the one contained in the sample
        SampleData::Integer(v) => {
            buffer[i..i + 4].copy_from_slice(&0u32.to_le_bytes());
            i += 4;
            buffer[i..i + 4].copy_from_slice(&v.to_le_bytes());
        }

        SampleData::Float(v) => {
            buffer[i..i + 4].copy_from_slice(&1u32.to_le_bytes());
            i += 4;
            buffer[i..i + 4].copy_from_slice(&v.to_le_bytes());
        }

        SampleData::Boolean(v) => {
            buffer[i..i + 4].copy_from_slice(&2u32.to_le_bytes());
            i += 4;
            buffer[i] = if v { 1 } else { 0 }
        }
    }
}

/// Decodes a Little Endian byte array back to a sample object
/// * `buffer`: A buffer which is any amount bytes long, which contains the Little Endian
fn decode(buffer: &[u8]) -> Result<Sample, ()> {
    let mut i = 0;

    // Check if the buffer is long enough to contain the byte array
    if buffer.len() < i + 4 {
        println!("Invalid Buffer. Does not contain a valid little endian for the header");
        return Err(())
    }

    // Decode the header
    let header = u32::from_le_bytes([
        buffer[i],
        buffer[i + 1],
        buffer[i + 2],
        buffer[i + 3]
    ]);

    i += 4;

    // Check if the header is fully zero
    if header != 0 {
        println!("Invalid Buffer. The header must be 0, and current is {header}");
        return Err(())
    }

    // Check if the buffer is long enough to contain the string length
    if buffer.len() < i + 4 {
        println!("Invalid Buffer. Does not contain a valid little endian for the string length");
        return Err(())
    }

    // Decode the string length
    let string_length = u32::from_le_bytes([
        buffer[i],
        buffer[i + 1],
        buffer[i + 2],
        buffer[i + 3]
    ]) as usize;

    i += 4;

    // Check if the buffer is long enough to contain the sample name
    if buffer.len() < i + string_length {
        println!("Invalid Buffer. Does not contain the whole string length");
        return Err(())
    }

    // Decode the sample name
    let string_name = match std::str::from_utf8(&buffer[i..i + string_length]) {
        Ok(value) => value.to_string(),
        Err(_value) => return Err(())
    };

    i += string_length;

    if buffer.len() < i + 4 {
        println!("Invalid Buffer. Does not contain a valid little endian for the sample data");
        return Err(())
    }

    // Check if the buffer is long enough to contain the data type
    let data_type = u32::from_le_bytes([
        buffer[i],
        buffer[i + 1],
        buffer[i + 2],
        buffer[i + 3]
    ]);

    i += 4;

    // Match between the different data types based on the value of the literal
    let data: SampleData = match data_type {
        0 => {
            if buffer.len() < i + 4 {
                return Err(())
            }

            let value: i32 = i32::from_le_bytes([ // Decode the Integer value
                buffer[i],
                buffer[i + 1],
                buffer[i + 2],
                buffer[i + 3]
            ]);

            SampleData::Integer(value)
        }

        1 => {
            if buffer.len() < i + 4 {
                return Err(())
            }

            let value: f32 = f32::from_le_bytes([ // Decode the Float value
                buffer[i],
                buffer[i + 1],
                buffer[i + 2],
                buffer[i + 3]
            ]);

            SampleData::Float(value)
        }

        2 => {
            if buffer.len() < i + 1 {
                return Err(())
            }

            let value: bool = match buffer[i] { // Decode the Boolean value
                0 => false,
                1 => true,
                _ => {
                    println!("Invalid Buffer. Contains SampleData of Boolean, but does not a valid byte for the boolean value");
                    return Err(())
                }
            };

            SampleData::Boolean(value)
        }

        _ =>  {
            println!("Invalid Buffer, Does not contain a valid little endian for the sample data");
            return Err(())
        }
    };

    Ok(Sample {
        name: string_name,
        data: data
    })
}

/// Receiver thread which handles receiving data from the transmitter, decoding the buffer, and sending it to the main thread for logging
fn receiver(tx: Sender<Sample>) {
    let socket = match std::net::UdpSocket::bind("10.0.0.1:34254") { // Creating the receiver socket
        Ok(socket) => socket,
        Err(error) => {
            println!("[Receiver] Failed to create UDP receiver: {}", error);
            return;
        }
    };

    println!("[Receiver] Successfully started, listening on port 5800...");

    let mut buffer = [0u8; 50]; // Create a buffer long enough to write the buffer sent from the transmitter 

    loop {
        let (amount, source) = match socket.recv_from(&mut buffer) {
            Ok(result) => {
                println!("Work pls");
                result
            }
            Err(error) => {
                println!("[Receiver] Failed to receive packet: {}", error);
                continue;
            }
        };

        println!("[Receiver] Received {} bytes from {}", amount, source);

        let sample = match decode(&buffer[..amount]) { // Decode the samples from the transmitter
            Ok(sample) => sample,
            Err(_) => {
                println!("[Receiver] Failed to decode the receiver buffer {buffer:?}");
                continue;
            }
        };

        match tx.send(sample) { // Send the samples to the main thread for logging
            Ok(_) => {}
            Err(error) => {
                println!("[Receiver] Failed to send sample to main: {}", error);
                break;
            }
        }
    }
}

/// The main thread, which is responsible for receiving samples from the receiver thread, and logging them in the console
fn main() {
    let (tx, rx) = channel::<Sample>(); // Create a Sender Receiver channel to send data from two threads

    std::thread::spawn(|| {
        receiver(tx);
    });

    loop {
        let sample = match rx.recv() { // Receive the sample from the receiver thread
            Ok(sample) => sample,

            Err(error) => {
                println!("[Main] Channel closed: {}", error);
                break;
            }
        };

        match &sample.data { // Match for each different data type to be logged nicely
            SampleData::Integer(value) => {
                println!("[Main] Sample \"{}\" | Integer | {}", sample.name, value);
            }

            SampleData::Float(value) => {
                println!("[Main] Sample \"{}\" | Float | {}", sample.name, value);
            }

            SampleData::Boolean(value) => {
                println!("[Main] Sample \"{}\" | Boolean | {}", sample.name, value);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encode_int() {
        let sample = Sample {
            name: "accel".into(),
            data: SampleData::Integer(5),
        };

        let mut bytes = [0u8; 21];
        encode(&sample, &mut bytes);
        assert_eq!(bytes, [0, 0, 0, 0, 5, 0, 0, 0, 97, 99, 99, 101, 108, 0, 0, 0, 0, 5, 0, 0, 0]);
    }

    #[test]
    fn test_encode_float() {
        let sample = Sample {
            name: "pos".into(),
            data: SampleData::Float(19.2),
        };

        let mut bytes = [0u8; 21];
        encode(&sample, &mut bytes);
        assert_eq!(bytes, [0, 0, 0, 0, 3, 0, 0, 0, 112, 111, 115, 1, 0, 0, 0, 154, 153, 153, 65, 0, 0]);
    }

    #[test]
    fn test_encode_boolean() {
        let sample = Sample {
            name: "velocity".into(),
            data: SampleData::Boolean(true),
        };

        let mut bytes = [0u8; 21];
        encode(&sample, &mut bytes);
        assert_eq!(bytes, [0, 0, 0, 0, 8, 0, 0, 0, 118, 101, 108, 111, 99, 105, 116, 121, 2, 0, 0, 0, 1]);
    }

    #[test]
    fn test_decode_int() {
        let sample = Sample {
            name: "accel".into(),
            data: SampleData::Integer(5),
        };

        let mut bytes = [0u8; 21];
        encode(&sample, &mut bytes);

        let result = decode(&bytes);
        let unwrapped = result.unwrap();

        assert_eq!(sample.data, unwrapped.data);
        assert_eq!(sample.name, unwrapped.name);
    }

    #[test]
    fn test_decode_float() {
        let sample = Sample {
            name: "accel".into(),
            data: SampleData::Float(5.2),
        };

        let mut bytes = [0u8; 21];
        encode(&sample, &mut bytes);

        let result = decode(&bytes);
        let unwrapped = result.unwrap();

        assert_eq!(sample.data, unwrapped.data);
        assert_eq!(sample.name, unwrapped.name);
    }

    #[test]
    fn test_decode_boolean() {
        let sample = Sample {
            name: "accel".into(),
            data: SampleData::Boolean(true),
        };

        let mut bytes = [0u8; 21];
        encode(&sample, &mut bytes);

        let result = decode(&bytes);
        let unwrapped = result.unwrap();

        assert_eq!(sample.data, unwrapped.data);
        assert_eq!(sample.name, unwrapped.name);
    }
}