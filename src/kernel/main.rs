#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    qemu_exit();
}

#[cfg(test)]
fn qemu_exit() -> ! {
    use core::ptr;

    unsafe {
        ptr::write_volatile(0x100000 as *mut u32, 0x3333);
    }
    loop {}
}

extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};
use kernel::{
    bio, console, kalloc, kmain, null, plic, println,
    proc::{self, scheduler, user_init, Cpus},
    trap, virtio_disk, vm,
};

static STARTED: AtomicBool = AtomicBool::new(false);

kmain!(main);

extern "C" fn main() -> ! {
    #[cfg(test)]
    test_main();

    let cpuid = unsafe { Cpus::cpu_id() };
    if cpuid == 0 {
        let initcode = include_bytes!(concat!(env!("OUT_DIR"), "/bin/_initcode"));
        console::init(); // console init
        println!("");
        println!("octox kernel is booting");
        println!("");
        null::init(); // null device init
        kalloc::init(); // physical memory allocator
        vm::kinit(); // create kernel page table
        vm::kinithart(); // turn on paging
        proc::init(); // process table
        trap::inithart(); // install kernel trap vector
        plic::init(); // set up interrupt controller
        plic::inithart(); // ask PLIC for device interrupts
        bio::init(); // buffer cache
        virtio_disk::init(); // emulated hard disk
        user_init(initcode);
        STARTED.store(true, Ordering::SeqCst);
    } else {
        while !STARTED.load(Ordering::SeqCst) {
            core::hint::spin_loop()
        }
        println!("hart {} starting", unsafe { Cpus::cpu_id() });
        vm::kinithart(); // turn on paging
        trap::inithart(); // install kernel trap vector
        plic::inithart(); // ask PLIC for device interrupts
    }
    scheduler()
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    use kernel::printf::panic_inner;
    panic_inner(info)
}
