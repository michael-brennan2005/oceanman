const ChainedStruct = @import("types.zig").ChainedStruct;
const PresentMode = @import("types.zig").PresentMode;
const Texture = @import("texture.zig").Texture;
const TextureView = @import("texture_view.zig").TextureView;
const Impl = @import("interface.zig").Impl;

pub const SwapChain = opaque {
    pub const Descriptor = extern struct {
        next_in_chain: ?*const ChainedStruct = null,
        label: ?[*:0]const u8 = null,
        usage: Texture.UsageFlags,
        format: Texture.Format,
        width: u32,
        height: u32,
        present_mode: PresentMode,
        /// deprecated
        implementation: u64 = 0,
    };

    pub inline fn configure(swap_chain: *SwapChain, format: Texture.Format, allowed_usage: Texture.UsageFlags, width: u32, height: u32) void {
        Impl.swapChainConfigure(swap_chain, format, allowed_usage, width, height);
    }

    pub inline fn getCurrentTextureView(swap_chain: *SwapChain) *TextureView {
        return Impl.swapChainGetCurrentTextureView(swap_chain);
    }

    pub inline fn present(swap_chain: *SwapChain) void {
        Impl.swapChainPresent(swap_chain);
    }

    pub inline fn reference(swap_chain: *SwapChain) void {
        Impl.swapChainReference(swap_chain);
    }

    pub inline fn release(swap_chain: *SwapChain) void {
        Impl.swapChainRelease(swap_chain);
    }
};
