const std = @import("std");
const gpu = @import("gpu");
const zmath = @import("zmath");
const glfw = @import("glfw");

const Mat = zmath.Mat;
const Vec = zmath.Vec;

const SceneResource = @import("resources.zig").SceneResource;
const LightingResource = @import("resources.zig").LightingResource;
const MeshResource = @import("resources.zig").MeshResource;

const Camera = @import("camera.zig").Camera;

pub const SceneDescripton = struct {
    pub const MeshDescription = struct {
        path: []u8,
        position: [3]f32 = .{ 0.0, 0.0, 0.0 },
        rotation: [3]f32 = .{ 0.0, 0.0, 0.0 },
        scale: [3]f32 = .{ 1.0, 1.0, 1.0 }
    };

    pub const LightDescription = struct {
        position: [3]f32 = .{ 0.0, 0.0, 0.0 },
        color: [3]f32 = .{ 0.0, 0.0, 0.0 }
    };

    light: LightDescription = .{},
    meshes: []MeshDescription,
};

const log = std.log.scoped(.loader);

pub const LoadResult = struct {
    meshes: []*MeshResource,
    lighting: *LightingResource
};

pub fn loadFromFile(gpa: std.mem.Allocator, device: *gpu.Device, path: []const u8) LoadResult {
    log.info("Creating scene from file {s}", .{path});
    var file = std.fs.cwd().openFile(path, .{ .mode = std.fs.File.OpenMode.read_only }) catch unreachable;
    defer file.close();
    const file_contents = file.readToEndAlloc(gpa, 1000000) catch unreachable;
    defer gpa.free(file_contents);    
    
    var parsed = std.json.parseFromSlice(SceneDescripton, gpa, file_contents, .{}) catch unreachable;
    
    const rads = std.math.degreesToRadians;
    
    var meshes_list = std.ArrayList(*MeshResource).init(gpa);
    for (parsed.meshes) |mesh| {
        var mesh_ptr = gpa.create(MeshResource) catch unreachable;

        const model_matrix = 
        zmath.mul(
            zmath.scaling(mesh.scale[0], mesh.scale[1], mesh.scale[2]),
        zmath.mul(
                zmath.rotationX(rads(f32, mesh.rotation[0])),
        zmath.mul(
                zmath.rotationY(rads(f32, mesh.rotation[1])),
        zmath.mul(
                zmath.rotationZ(rads(f32, mesh.rotation[2])),
                zmath.translation(mesh.position[0], mesh.position[1], mesh.position[2]),
        ))));
        
        mesh_ptr.* = MeshResource.init(gpa, device, mesh.path, model_matrix) catch unreachable;
        meshes_list.append(mesh_ptr) catch unreachable;
    }

    var lighting_resource = gpa.create(LightingResource) catch unreachable;
    lighting_resource.* = LightingResource.init(device, parsed.light.position, parsed.light.color);
    log.info("Loading from file {s} complete!", .{path});
    return .{
        .meshes = meshes_list.toOwnedSlice() catch unreachable,
        .lighting = lighting_resource
    };
} 
