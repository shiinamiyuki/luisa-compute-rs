# luisa-compute-rs
Rust frontend to LuisaCompute and more! (WIP) ⚠ A stable version will be released with the next version of LuisaCompute ⚠. See [next](https://github.com/LuisaGroup/luisa-compute-rs/tree/next) branch for ongoing development.

## Table of Contents
* [Overview](#overview)
    + [Embedded Domain-Specific Language](#embedded-domain-specific-language)
    + [Automatic Differentiation](#automatic-differentiation)
    + [A CPU backend](#cpu-backend)
    + [IR Module for EDSL](#ir-module)
    + [Debuggability](#debuggability)
* [Usage](#usage)
    + [Variables and Expressions](#variables-and-expressions)
    + [Control Flow](#control-flow)
    + [Custom Data Types](#custom-data-types)
    + [Polymorphism](#polymorphism)
    + [Autodiff](#autodiff)
    + [Custom Operators](#custom-operators)
    + [Kernel](#kernel)
* [Advanced Usage](#advanced-usage)

* [Safety](#safety)

## Example
```rust
use luisa::prelude::*;
use luisa_compute as luisa;

fn main() {
    init();
    let device = create_cpu_device().unwrap();
    let x = device.create_buffer::<f32>(1024).unwrap();
    let y = device.create_buffer::<f32>(1024).unwrap();
    let z = device.create_buffer::<f32>(1024).unwrap();
    x.view(..).fill_fn(|i| i as f32);
    y.view(..).fill_fn(|i| 1000.0 * i as f32);
    let kernel = device
        .create_kernel::<(Buffer<f32>,)>(&|buf_z| {
            // z is pass by arg
            let buf_x = x.var(); // x and y are captured
            let buf_y = y.var();
            let tid = dispatch_id().x();
            let x = buf_x.read(tid);
            let y = buf_y.read(tid);
            buf_z.write(tid, x + y);
        })
        .unwrap();
    kernel.dispatch([1024, 1, 1], &z).unwrap();
    let z_data = z.view(..).copy_to_vec();
    println!("{:?}", &z_data[0..16]);
}


```
## Overview
### Embedded Domain-Specific Language
We provided an Rust-flavored implementation of LuisaCompute EDSL that tightly integrates with Rust language via traits and proc-macros.

### Automatic Differentiation
We implemented a source-to-source reverse mode automatic differentiation that supports complex control flow.

The autodiff works tightly with builtin functions and the type system. Instead of implementing every function using basic arithmetic operations and apply autodiff on it, all builtin functions are differentiated using efficient VJP formulae.

### CPU Backend
This crate also provides a CPU backend implementation in Rust that will eventually replace the current LLVM backend in LuisaCompute. This backend emphasizes on debuggability, flexibility and as well as safety.

### IR Module
The EDSL and code generation are built atop of an SSA-based IR module. The IR module is in a separate crate and can be used to implement other backends and IR transformation such as autodiff.

### Debuggability
The CPU backend is designed to be debuggable. If needed, it will perform runtime checks to detect errors such as out-of-bound access, bindless array type mismatch, etc. It will display error message containing the **host** stacktrace for pinpointing the error location.

## Usage
To get started, add the following to your `Cargo.toml`:
```toml
[dependencies]
luisa_compute = { git= "https://github.com/LuisaGroup/luisa-compute-rs.git"}
```
Then added the following to your files:
```rust
use luisa_compute as luisa;
use luisa::prelude::*;
```
### Variables and Expressions
There are six basic types in EDSL. `bool`, `i32`, `u32`, `i64`, `u64`, `f32`. (`f64` support might be added to CPU backend).
For each type, there are two EDSL proxy objects `Expr<T>` and `Var<T>`. `Expr<T>` is an immutable object that represents a value. `Var<T>` is a mutable object that represents a variable. `Expr<T>` can be converted to `Var<T>` by calling `Var<T>::load()`.
All operations except load/store should be performed on `Expr<T>`. `Var<T>` can only be used to load/store values. While `Expr<T>` and `Var<T>` are sufficent in most cases, it cannot be placed in an `impl` block. To do so, the exact name of these proxies are needed.
```rust
Expr<Bool> == Bool, Var<Bool> == BoolVar
Expr<f32> == Float32, Var<f32> == Float32Var
Expr<i32> == Int32, Var<i32> == Int32Var
Expr<u32> == UInt32, Var<u32> == UInt32Var
Expr<i64> == Int64, Var<i64> == Int64Var
Expr<u64> == UInt64, Var<u64> == UInt64Var
```

As in the C++ EDSL, we additionally supports the following vector/matrix types. Their proxy types are `XXXExpr` and `XXXVar`:

```rust
BVec2 // bool2 in C++
BVec3 // bool3 in C++
BVec4 // bool4 in C++
Vec2 // float2 in C++
Vec3 // float3 in C++
Vec4 // float4 in C++
IVec2 // int2 in C++
IVec3 // int3 in C++
IVec4 // int4 in C++
UVec2 // uint2 in C++
UVec3 // uint3 in C++
UVec4 // uint4 in C++
Mat2 // float2x2 in C++
Mat3 // float3x3 in C++
Mat4 // float4x4 in C++
```
Array types `[T;N]` are also supported and their proxy types are `ArrayExpr<T, N>` and `ArrayVar<T, N>`.

### Control Flow
If, while, break, continue are supported. Note that `if` and `switch` works similar to native Rust `if` and `match` in that values can be returned at the end of the block.

```rust
if_!(cond, { /* then */});
if_!(cond, { /* then */}, { /* else */});
if_!(cond, { value_a }, { value_b })
while_!(cond, { /* body */});
break_();
continue_();
let (x,y) = switch::<(Expr<i32>, Expr<f32>)>(value)
    .case(1, || { ... })
    .case(2, || { ... })
    .default(|| { ... })
    .finish();
```

### Custom Data Types
To add custom data types to the EDSL, simply derive from `luisa::Value` macro. Note that `#[repr(C)]` is required for the struct to be compatible with C ABI. The proxy types are `XXXExpr` and `XXXVar`:

```rust
#[derive(Copy, Clone, Default, Debug, Value)]
#[repr(C)]
pub struct MyVec2 {
    pub x: f32,
    pub y: f32,
}

let v: Var<MyVec2> = local::<MyVec2>();
let v_ld: Expr<MyVec2> = v.load();
let v_x = v_ld.x();
let v_ld = v_ld.set_x(v_x + 1.0); // v_ld.x += 1.0
// or
v.set_x(v_ld.x() + 1.0);

```
### Polymorphism
We prvoide the similar `Polymorphic<dyn Trait>` construct as in the C++ DSL.
```rust
trait Area {
    fn area(&self) -> Float32;
}
#[derive(Value, Clone, Copy)]
#[repr(C)]
pub struct Circle {
    radius: f32,
}
impl Area for CircleExpr {
    fn area(&self) -> Float32 {
        PI * self.radius() * self.radius()
    }
}
impl_polymorphic!(Area, Circle);

let circles = device.create_buffer(..).unwrap()'
let mut poly_area: Polymorphic<dyn Area> = Polymorphic::new();
poly_area.register(&circles);
let area = poly_area.dispatch(tag, index, |obj|{
    obj.area()
});
```

### Autodiff
Autodiff code should be enclosed in the `autodiff` call. The `requires_grad` call is used to mark the variables that need to be differentiated. Any type including user defined ones can receive gradients. The `backward` call triggers the backward pass. Subsequent calls to `gradient` will return the gradient of the variable passed in. User can also supply custom gradients with `backward_with_grad`.

Note: Only one backward call is allowed in a single autodiff block. The autodiff block does not return any value. To store any side effects, use of local variables or buffers is required.

```rust
autodiff(||{
    let v: Expr<Vec3> = buf_v.read(..);
    let m: Expr<Mat3> = buf_m.read(..);
    requires_grad(v);
    requires_grad(m);
    let z = v.dot(m * v) * 0.5;
    backward(z);
    let dv = gradient(dv);
    let dm = gradient(dm);
    buf_dv.write(.., dv);
    buf_dm.write(.., dm);
});


```

### Custom Operators
LuisaCompute supports injecting arbitrary user code to implement a custom operator. This is handled differently on different backends.
On CPU backends, user can directly pass a closure to the kernel. The closure needs to have a `Fn(&mut T)` signature where it modifies the argument inplace. The EDSL frontend would then wrap the closure into a `T->T` function object.

```rust
#[derive(Clone, Copy, Value, Debug)]
#[repr(C)]
pub struct MyAddArgs {
    pub x: f32,
    pub y: f32,
    pub result: f32,
}
let my_add = CpuFn::new(|args: &mut MyAddArgs| {
    args.result = args.x + args.y;
});

let args = MyAddArgsExpr::new(x, y, Float32::zero());
let result = my_add.call(args);

```

### Kernel
A kernel can be written in a closure or a function. The closure/function should have a `Fn(/*args*/)->()` signature, where the args are taking the `Var` type of resources, such as `BufferVar<T>`, `Tex2D<T>`, etc.

Note: `Device::create_kernel` takes a tuple of types as its generic parameter. If the kernel takes a single argument, it is required to use `create_kernel::<(Type,)>` instead of `create_kernel::<Type>`.

```rust
let kernel = device.create_kernel::<(Arg0, Arg1, ...)>(&|/*args*/| {
    /*body*/
}).unwrap();
kernel.dispatch([/*dispatch size*/], &arg0, &arg1, ...).unwrap();
```
There are two ways to pass arguments to a kernel: by arguments or by capture.
```rust
let captured:Buffer<f32> = device.create_buffer(...).unwrap();
let kernel = device.create_kernel::<(BufferVar<f32>, )>(arg| {
    let v = arg.read(..);
    let u = captured.var().read(..);
})).unwrap();
```
User can pass a maximum of 16 arguments to kernel and unlimited number of captured variables. If more than 16 arguments are needed, user can pack them into a struct and pass the struct as a single argument.
```rust
#[derive(KernelArg)]
pub struct BufferPair {
    a:Buffer<f32>,
    b:Buffer<f32>
}
let kernel = device.create_kernel::<(BufferPair, )>(&|| {
    // ...
}).unwrap();
let a = device.create_buffer(...).unwrap();
let b = device.create_buffer(...).unwrap();
let pair = BufferPair{a,b};
kernel.dispatch([...], &packed).unwrap();
let BufferPair{a, b} = packed; // unpack if you need to use them later
```
## Advanced Usage
Note that the IR module has a public interface. If needed, user can implement their own DSL syntax sugar. Every EDSL object implements either `Aggregate` or `FromNode` trait, which allows any EDSL type to be destructured into its underlying IR nodes and reconstructed from them.

TODO

## Safety
### API
Host-side safety: The API aims to be 100% safe on host side. However, the safety of async operations are gauranteed via staticly know sync points (such as `Stream::submit_and_sync`). If fully dynamic async operations are needed, user need to manually lift the liftime and use unsafe code accordingly.

Device-side safety: Due to the async nature of device-side operations. It is both very difficult to propose a safe **host** API that captures **device** resource lifetime. While device-side safety isn't guaranteed at compile time, on `cpu` backend runtime checks will catch any illegal memory access/racing condition during execution. However, for other backends such check is either too expensive or impractical and memory errors would result in undefined behavior instead.

### Backend
Safety checks such as OOB is generally not available for GPU backends. As it is difficult to produce meaningful debug message in event of a crash. However, the Rust backend provided in the crate contains full safety checks and is recommended for debugging.
