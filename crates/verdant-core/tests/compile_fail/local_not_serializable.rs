use verdant_core::sovereignty::Local;
use verdant_core::types::SensorReading;

fn main() {
    let local = Local::new(SensorReading {
        temperature: 2150,
        humidity: 6500,
        soil_moisture: 4200,
        pressure: 101325,
        pressure_delta: -50,
        light: 850,
    });

    // This must fail to compile: Local<T> does not implement Serialize.
    let _bytes = postcard::to_allocvec(&local).unwrap();
}
