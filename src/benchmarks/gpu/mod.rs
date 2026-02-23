//! Point d’entrée commun pour les backends GPU.
//! Fournit des fonctions de détection runtime et un dispatch
//! vers le backend choisi.

#[cfg(feature = "vulkan")]
mod vulkan;
#[cfg(feature = "metal")]
mod metal;
#[cfg(feature = "dx12")]
mod dx12;
#[cfg(feature = "opengl")]
mod opengl;

#[cfg(feature = "vulkan")]
pub fn supports_vulkan() -> bool {
    ash::Entry::linked().is_ok()
}
#[cfg(not(feature = "vulkan"))]
pub fn supports_vulkan() -> bool {
    false
}

#[cfg(feature = "metal")]
pub fn supports_metal() -> bool {
    // metal‑rs expose `Device::system_default()` qui renvoie Option<Device>
    metal::Device::system_default().is_some()
}
#[cfg(not(feature = "metal"))]
pub fn supports_metal() -> bool {
    false
}

#[cfg(feature = "dx12")]
pub fn supports_dx12() -> bool {
    // windows‑rs expose des fonctions COM pour DirectX12 ; on teste simplement la présence de la DLL.
    std::path::Path::new(r"C:\Windows\System32\d3d12.dll").exists()
}
#[cfg(not(feature = "dx12"))]
pub fn supports_dx12() -> bool {
    false
}

#[cfg(feature = "opengl")]
pub fn supports_opengl() -> bool {
    // Utilise glutin pour créer un contexte windowless minimal.
    glutin::ContextBuilder::new()
        .build_headless(glutin::event_loop::EventLoop::new(), glutin::dpi
