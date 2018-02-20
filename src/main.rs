extern crate winapi;
extern crate winit;
extern crate wio;

use com::{OutParam, ToResult, HResult};
use std::ptr;
use winapi::shared::dxgi1_2::DXGI_SWAP_CHAIN_DESC1;
use winapi::shared::dxgi1_2::IDXGIFactory2;
use winapi::shared::minwindef::{TRUE, FALSE};
use winapi::shared::windef::HWND;
use winapi::um::d3d11::ID3D11Device;
use winapi::um::dcomp::IDCompositionDevice;
use winapi::um::dcomp::IDCompositionTarget;
use winapi::um::dcomp::IDCompositionVisual;
use winapi::um::unknwnbase::IUnknown;
use winit::os::windows::WindowExt;
use wio::com::ComPtr;

mod com;

fn main() {
    let mut events_loop = winit::EventsLoop::new();
    let window = winit::WindowBuilder::new()
        .with_title("Hello, world!")
        .with_dimensions(1024, 768)
        .build(&events_loop)
        .unwrap();

    let _composition = unsafe {
        let dc = DirectComposition::initialize(window.get_hwnd() as _).unwrap();
        dc.create_swap_chain(300, 200).unwrap();
    };

    if std::env::var_os("INIT_ONLY").is_some() {
        return
    }

    events_loop.run_forever(|event| match event {
        winit::Event::WindowEvent { event: winit::WindowEvent::Closed, .. } => {
            winit::ControlFlow::Break
        }
        _ => winit::ControlFlow::Continue,
    });
}

/// https://msdn.microsoft.com/en-us/library/windows/desktop/hh449180(v=vs.85).aspx

#[allow(unused)]
struct DirectComposition {
    d3d_device: ComPtr<ID3D11Device>,
    dxgi_factory: ComPtr<IDXGIFactory2>,
    composition_device: ComPtr<IDCompositionDevice>,
    composition_target: ComPtr<IDCompositionTarget>,
    root_visual: ComPtr<IDCompositionVisual>,
}

impl DirectComposition {
    unsafe fn initialize(hwnd: HWND) -> HResult<Self> {
        let mut feature_level_supported = 0;

        let d3d_device = ComPtr::new_with(|ptr_ptr| winapi::um::d3d11::D3D11CreateDevice(
            ptr::null_mut(),
            winapi::um::d3dcommon::D3D_DRIVER_TYPE_HARDWARE,
            ptr::null_mut(),
            winapi::um::d3d11::D3D11_CREATE_DEVICE_BGRA_SUPPORT |
            if cfg!(debug_assertions) {
                winapi::um::d3d11::D3D11_CREATE_DEVICE_DEBUG
            } else {
                0
            },
            ptr::null_mut(),
            0,
            winapi::um::d3d11::D3D11_SDK_VERSION,
            ptr_ptr,
            &mut feature_level_supported,
            ptr::null_mut(),
        ))?;

        let dxgi_device = d3d_device.cast::<winapi::shared::dxgi::IDXGIDevice>()?;

        // https://msdn.microsoft.com/en-us/library/windows/desktop/hh404556(v=vs.85).aspx#code-snippet-1
        // “Because you can create a Direct3D device without creating a swap chain,
        //  you might need to retrieve the factory that is used to create the device
        //  in order to create a swap chain.”
        let adapter = ComPtr::new_with(|ptr_ptr| dxgi_device.GetAdapter(ptr_ptr))?;
        let dxgi_factory = ComPtr::<IDXGIFactory2>::new_with_uuid(|uuid, ptr_ptr| {
            adapter.GetParent(uuid, ptr_ptr)
        })?;

        // Create the DirectComposition device object.
        let composition_device = ComPtr::<IDCompositionDevice>::new_with_uuid(|uuid, ptr_ptr| {
            winapi::um::dcomp::DCompositionCreateDevice(&*dxgi_device, uuid,ptr_ptr)
        })?;

        // Create the composition target object based on the
        // specified application window.
        let composition_target = ComPtr::new_with(|ptr_ptr| {
            composition_device.CreateTargetForHwnd(hwnd, TRUE, ptr_ptr)
        })?;

        let root_visual = ComPtr::new_with(|ptr_ptr| composition_device.CreateVisual(ptr_ptr))?;
        composition_target.SetRoot(&*root_visual).to_result()?;

        Ok(DirectComposition {
            d3d_device, dxgi_factory, composition_device, composition_target, root_visual,
        })
    }

    unsafe fn create_swap_chain(&self, width: u32, height: u32)
        -> HResult<ComPtr<winapi::shared::dxgi1_2::IDXGISwapChain1>>
    {
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width,
            Height: height,
            Format: winapi::shared::dxgiformat::DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: FALSE,
            SampleDesc: winapi::shared::dxgitype::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: winapi::shared::dxgitype::DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: winapi::shared::dxgi1_2::DXGI_SCALING_STRETCH,
            SwapEffect: winapi::shared::dxgi::DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode:  winapi::shared::dxgi1_2::DXGI_ALPHA_MODE_IGNORE,
            Flags: 0,
        };
        ComPtr::<winapi::shared::dxgi1_2::IDXGISwapChain1>::new_with(|ptr_ptr| {
            self.dxgi_factory.CreateSwapChainForComposition(
                self.d3d_device.as_raw() as *mut IUnknown,
                &desc,
                ptr::null_mut(),
                ptr_ptr,
            )
        })
    }
}
