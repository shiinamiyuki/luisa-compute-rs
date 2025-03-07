use luisa_compute as luisa;
use luisa::*;
fn main() {
    luisa::init();
    luisa::init_logger();

    let device = luisa::create_cpu_device().unwrap();
    let x = device.create_buffer::<f32>(1024).unwrap();
    let y = device.create_buffer::<f32>(1024).unwrap();
    let dx = device.create_buffer::<f32>(1024).unwrap();
    let dy = device.create_buffer::<f32>(1024).unwrap();
    x.fill_fn(|i| i as f32);
    y.fill_fn(|i| 1.0 + i as f32);
    let shader = device
        .create_kernel::<(Buffer<f32>, Buffer<f32>, Buffer<f32>, Buffer<f32>)>(
            &|buf_x: BufferVar<f32>,
              buf_y: BufferVar<f32>,
              buf_dx: BufferVar<f32>,
              buf_dy: BufferVar<f32>| {
                let tid = dispatch_id().x();
                let x = buf_x.read(tid);
                let y = buf_y.read(tid);
                autodiff(|| {
                    requires_grad(x);
                    requires_grad(y);
                    let z = x * y.sin();
                    backward(z);
                    buf_dx.write(tid, gradient(x));
                    buf_dy.write(tid, gradient(y));
                });
            },
        )
        .unwrap();
    shader.dispatch([1024, 1, 1], &x.view(..), &y, &dx, &dy).unwrap();
    let dx = dx.copy_to_vec();
    println!("{:?}", &dx[0..16]);
    let dy = dy.copy_to_vec();
    println!("{:?}", &dy[0..16]);
}
