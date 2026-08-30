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

fn encode(sample: &Sample, buffer: &mut [u8]) {
    let mut i = 0;

    buffer[i..i + 4].copy_from_slice(&0u32.to_le_bytes());
    i += 4;

    let string_length = sample.name.len() as u32;
    buffer[i..i + 4].copy_from_slice(&string_length.to_le_bytes());
    i += 4;

    let name_bytes = sample.name.as_bytes();
    buffer[i..i + name_bytes.len()].copy_from_slice(name_bytes);
    i += name_bytes.len();

    match sample.data {
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

fn decode(buffer: &[u8]) -> Result<Sample, ()> {
    let mut i = 0;

    if buffer.len() < i + 4 {
        println!("Invalid Buffer. Does not contain a valid little endian for the header");
        return Err(())
    }

    let header = u32::from_le_bytes([
        buffer[i],
        buffer[i + 1],
        buffer[i + 2],
        buffer[i + 3]
    ]);

    i += 4;

    if header != 0 {
        println!("Invalid Buffer. The header must be 0, and current is {header}");
        return Err(())
    }

    if buffer.len() < i + 4 {
        println!("Invalid Buffer. Does not contain a valid little endian for the string length");
        return Err(())
    }

    let string_length = u32::from_le_bytes([
        buffer[i],
        buffer[i + 1],
        buffer[i + 2],
        buffer[i + 3]
    ]) as usize;

    i += 4;


    if buffer.len() < i + string_length {
        println!("Invalid Buffer. Does not contain the whole string length");
        return Err(())
    }

    let string_name = match std::str::from_utf8(&buffer[i..i + string_length]) {
        Ok(value) => value.to_string(),
        Err(_value) => return Err(())
    };

    i += string_length;

    if buffer.len() < i + 4 {
        println!("Invalid Buffer. Does not contain a valid little endian for the sample data");
        return Err(())
    }

    let data_type = u32::from_le_bytes([
        buffer[i],
        buffer[i + 1],
        buffer[i + 2],
        buffer[i + 3]
    ]);

    i += 4;

    let data: SampleData = match data_type {
        0 => {
            if buffer.len() < i + 4 {
                return Err(())
            }

            let value: i32 = i32::from_le_bytes([
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

            let value: f32 = f32::from_le_bytes([
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

            let value: bool = match buffer[i] {
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

    println!("Valid Buffer Received. Decoded with name of {string_name} and sample data of {data:?}");

    Ok(Sample {
        name: string_name,
        data: data
    })
}

fn listen() {
    let socket = match std::net::UdpSocket::bind("127.0.0.1:5800") {
        Ok(socket) => socket,
        Err(error) => {
            println!("Failed to bind UDP socket: {}", error);
            return;
        }
    };

    println!("Listening on port 5800");

    let mut buffer = [0u8; 1024];

    loop {
        let (amount, source) = match socket.recv_from(&mut buffer) {
            Ok(result) => result,
            Err(error) => {
                println!("Failed to receive packet: {}", error);
                continue;
            }
        };

        let sample = match decode(&buffer[..amount]) {
            Ok(sample) => sample,
            Err(_) => {
                println!("Received invalid sample");
                continue;
            }
        };

        match sample.data {
            SampleData::Integer(value) => {
                println!("{} {} {}", source.ip(), sample.name, value);
            }

            SampleData::Float(value) => {
                println!("{} {} {}", source.ip(), sample.name, value);
            }

            SampleData::Boolean(value) => {
                println!("{} {} {}", source.ip(), sample.name, value);
            }
        }
    }
}

fn main() {
    std::thread::spawn(|| {
        listen();
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let socket = match std::net::UdpSocket::bind("127.0.0.1:0") {
        Ok(socket) => socket,
        Err(error) => {
            println!("Failed to create sender: {}", error);
            return;
        }
    };

    let sample = Sample {
        name: "accel".into(),
        data: SampleData::Boolean(true),
    };

    let mut buffer = [0u8; 21];

    encode(&sample, &mut buffer);

    match socket.send_to(&buffer, "127.0.0.1:5800") {
        Ok(amount) => {
            println!("Sent {} bytes", amount);
        }

        Err(error) => {
            println!("Failed to send packet: {}", error);
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(1000));
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