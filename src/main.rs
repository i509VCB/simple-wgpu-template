use wgpu::{Instance, BackendBit, RequestAdapterOptions, PowerPreference, Surface, Adapter, DeviceDescriptor, RequestDeviceError, SwapChain, SwapChainDescriptor, TextureUsage, PresentMode, Device, TextureFormat, Queue, SwapChainError, SwapChainFrame, CommandEncoderDescriptor, CommandEncoder, RenderPassColorAttachmentDescriptor, Color, LoadOp, Operations, RenderPassDescriptor};
use pollster::block_on;
use winit::event_loop::{EventLoop, ControlFlow};
use winit::window::{WindowBuilder, Window};
use winit::dpi::PhysicalSize;
use winit::error::OsError;
use winit::event::{Event, WindowEvent};
use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};

fn main() {
	// Starting is simple:
	// - Create the window
	// - Create the render state
	// - Create the initial scene
	// - Run

	let event_loop = EventLoop::new();
	let window = create_window(&event_loop).expect("Failed to create window");
	let mut state = RenderState::new(window).expect("Failed to create render state");
	let mut scene = Scene {};

	run(event_loop, state, scene);
}

/// Creates a window.
/// Use this to customize the title and default size.
fn create_window(event_loop: &EventLoop<()>) -> Result<Window, OsError> {
	WindowBuilder::new()
		.with_title("wgpu-template")
		.with_inner_size(PhysicalSize::new(1280, 720))
		.build(event_loop)
}

/// State used to render a frame.
/// Contains handles to the physical device and swapchain.
struct RenderState {
	window: Window,
	instance: Instance,
	surface: Surface,
	device: Device,
	queue: Queue,
	swapchain_descriptor: SwapChainDescriptor,
	swapchain: SwapChain,
}

/// The scene being rendered.
/// Generally objects such as models and shaders should be stored here as you may load and unload
/// them as needed.
struct Scene {
}

impl RenderState {
	pub fn new(window: Window) -> Result<RenderState, InitError> {
		let instance = Instance::new(BackendBit::PRIMARY);
		let surface = unsafe { instance.create_surface(&window) };

		let request_options = RequestAdapterOptions {
			power_preference: PowerPreference::HighPerformance,
			compatible_surface: Some(&surface)
		};

		let adapter = block_on(instance.request_adapter(&request_options));
		let adapter = adapter.ok_or(InitError::NoAdapter)?;

		let device_descriptor = DeviceDescriptor::default();
		let (device, queue) = block_on(adapter.request_device(&device_descriptor, None))
			.map_err(|e| InitError::RequestDevice(e))?;

		let texture_format = adapter.get_swap_chain_preferred_format(&surface);

		let mut swapchain_descriptor = SwapChainDescriptor {
			usage: TextureUsage::RENDER_ATTACHMENT,
			format: texture_format,
			width: 0, // Size will be set at swapchain creation time.
			height: 0,
			present_mode: PresentMode::Fifo, // TODO: Is this the best presentation mode?
		};

		let swapchain = create_swapchain(
			&mut swapchain_descriptor,
			&window,
			&device,
			&surface
		);

		Ok(RenderState {
			window,
			instance,
			surface,
			device,
			queue,
			swapchain_descriptor,
			swapchain
		})
	}

	fn recreate_swapchain(&mut self) {
		self.swapchain = create_swapchain(
			&mut self.swapchain_descriptor,
			&self.window,
			&self.device,
			&self.surface
		);
	}
}

fn create_swapchain(
	descriptor: &mut SwapChainDescriptor,
	window: &Window,
	device: &Device,
	surface: &Surface
) -> SwapChain {
	let window_size = window.inner_size();

	descriptor.width = window_size.width;
	descriptor.height = window_size.height;

	device.create_swap_chain(surface, descriptor)
}

#[derive(Debug)]
enum InitError {
	NoAdapter,
	RequestDevice(RequestDeviceError)
}

/// Result of rendering a frame
enum RenderResult {
	/// Nothing in particular is abnormal
	Ok,
	/// Swapchain is invalid and needs to be recreated.
	RecreateSwapchain,
	/// Timed out while trying to obtain frame.
	TimedOut,
}

/// Main event loop
/// This is where events will be dispatched to the window, frames are called to be rendered and
/// where logic may be run.
fn run(event_loop: EventLoop<()>, mut state: RenderState, mut scene: Scene) -> ! {
	event_loop.run(move |event, _, control_flow| {
		match event {
			Event::WindowEvent { event, .. } => {
				match event {
					// Resizing means we need a new swapchain
					WindowEvent::Resized(_) => {
						&state.recreate_swapchain();
					}
					WindowEvent::CloseRequested => {
						println!("Exiting");
						*control_flow = ControlFlow::Exit;
					}
					_ => {}
				}
			}
			Event::RedrawRequested(_) => {
				match render(&mut state, &mut scene) {
					RenderResult::Ok => {}
					RenderResult::RecreateSwapchain => state.recreate_swapchain(),
					RenderResult::TimedOut => eprintln!("Timed out obtaining frame."),
				}
			}
			Event::MainEventsCleared => {
				// Request redraw for the next frame.
				&state.window.request_redraw();

				// Generally you will put logic ticking code in the event loop here...
				// A frame timer may be nice to account for here.
			}
			_ => {}
		}
	});
}

/// Renders a frame.
/// The main purpose of this method is to deal with the possible states that could occur when
/// obtaining the frame to render and create the command encoder.
fn render(state: &mut RenderState, scene: &Scene) -> RenderResult {
	let frame = state.swapchain.get_current_frame();

	match frame {
		Ok(frame) => {
			let encoder_descriptor = CommandEncoderDescriptor { label: None };
			let mut encoder = state.device.create_command_encoder(&encoder_descriptor);

			render_contents(state, scene, &frame, &mut encoder);

			state.queue.submit(Some(encoder.finish()));

			RenderResult::Ok
		}
		Err(e) => {
			match e {
				SwapChainError::Timeout => RenderResult::TimedOut,
				SwapChainError::Outdated | SwapChainError::Lost => RenderResult::RecreateSwapchain,
				SwapChainError::OutOfMemory => panic!("Out of memory!"),
			}
		}
	}
}

/// Here you can render things as the state and encoders have been setup.
///
/// You are given the scene for context.
fn render_contents(
	state: &mut RenderState,
	scene: &Scene,
	frame: &SwapChainFrame,
	encoder: &mut CommandEncoder
) {
	render_colored_background(state, scene, frame, encoder);
}

/// Render a colored background that transitions between two colors.
fn render_colored_background(
	_state: &mut RenderState,
	_scene: &Scene,
	frame: &SwapChainFrame,
	encoder: &mut CommandEncoder
) {
	const BLACK: Color = Color {
		r: 0.0,
		g: 0.0,
		b: 0.0,
		a: 1.0
	};

	const WHITE: Color = Color {
		r: 1.0,
		g: 1.0,
		b: 1.0,
		a: 1.0
	};

	/// Linear interpolation between two colors
	fn lerp_color(a: Color, b: Color, delta: f64) -> Color {
		Color {
			r: (1.0 - delta) * a.r + delta * b.r,
			g: (1.0 - delta) * a.g + delta * b.g,
			b: (1.0 - delta) * a.b + delta * b.b,
			a: 1.0
		}
	}

	let duration = SystemTime::now().duration_since(UNIX_EPOCH)
		.unwrap_or(Duration::from_secs(0));

	let modulo = duration.as_millis() % 20000;
	let delta = (f64::cos((0.001) * modulo as f64) * 0.5) + 0.5;

	let render_pass_color_attachment = RenderPassColorAttachmentDescriptor {
		attachment: &frame.output.view,
		resolve_target: None,
		ops: Operations {
			load: LoadOp::Clear(lerp_color(BLACK, WHITE, delta)),
			store: true,
		},
	};

	let render_pass_descriptor = RenderPassDescriptor {
		label: None,
		color_attachments: &[render_pass_color_attachment],
		depth_stencil_attachment: None,
	};

	// When dropped, render pass is ended.
	let _render_pass = encoder.begin_render_pass(&render_pass_descriptor);
}
