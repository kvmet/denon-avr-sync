use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use libpulse_binding::context::Context;
use libpulse_binding::mainloop::standard::MainLoop;
use libpulse_binding::volume::Volume;
use telnet::Telnet;
use toml::Toml;
use tokio::runtime::Runtime;

fn main() {
    // Initialize the tokio runtime for async operations
    let rt = Runtime::new().unwrap();

    // Shared volume state
    let volume_level = Arc::new(Mutex::new(None));

    // Start the Telnet communication task
    let volume_level_clone = Arc::clone(&volume_level);
    rt.spawn(async move {
        telnet_task(volume_level_clone).await;
    });

    // Setup PulseAudio mainloop
    let mut mainloop = Mainloop::new().expect("Failed to create mainloop");
    let context = Context::new(&mut mainloop, "Volume Sync").expect("Failed to create context");

    // Connect to the server
    context
        .connect(None, libpulse_binding::context::flags::NOFLAGS, None)
        .expect("Failed to connect context");

    // Listen for volume change events
    context.set_subscribe_callback(Some(Box::new(move |_, event| {
        if let Some(Operation::SinkInput) = event.operation {
            update_volume_level(&context, &volume_level);
        }
    })));
    context.subscribe(subscribe::MASK_SINK_INPUT, |_| {}).unwrap();

    // Run the mainloop
    mainloop.run().unwrap();
}

fn update_volume_level(context: &Context, volume_level: &Arc<Mutex<Option<Volume>>>) {
    let state = context.get_state();
    if state == libpulse_binding::context::State::Ready {

        let introspect = &context.introspect();
        introspect.get_sink_info_by_index(0, move |info| {
            if let Some(info) = info {
                let volume = info.volume.get().avg();
                let mut volume_level_guard = volume_level.lock().unwrap();
                *volume_level_guard = Some(volume);
                println!("Volume updated: {:?}", volume); // Print the volume to check
            }
        });
    }
}

async fn telnet_task(volume_level: Arc<Mutex<Option<Volume>>>) {
  let mut connection = Telnet::connect(("192.168.0.182",23), 256).unwrap();
  loop {
    {
      let volume_level_guard = volume_level.lock().unwrap();
      if let Some(vol) = *volume_level_guard {
        let vol_command = format!("MV{}\r\n",vol.0);
        let _ = connection.write(vol_command.as_bytes()).unwrap();
        println!("Sent volume command: {}",vol_command);
      }
    }
    thread::sleep(std::time::duration::from_secs(1));
  }
}
