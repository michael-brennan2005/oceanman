const std = @import("std");
const model3d = @import("model3d");


fn readFn(filename: [*c]u8, size: [*c]c_uint) callconv(.C) [*c]u8 {
    std.debug.print("Model loader wants to read {s} size {?*}", .{filename, size});
    var cool: u8 = 5;
    return &cool;
}
pub const Model = struct {
    buffer: []f32,
    texture: ?[]u8,
    texture_width: u16 = 0,
    texture_height: u16 = 0,

    pub fn createFromFile(gpa: std.mem.Allocator, path: []const u8, textured: bool) !Model {
        // FIXME: cmon man - sentinel b.s
        var file = std.fs.cwd().openFile(path, .{}) catch unreachable;
        defer file.close();
        const file_contents = file.readToEndAlloc(gpa, std.math.inf_u64) catch unreachable;
        defer gpa.free(file_contents);    
        const file_sentinel = gpa.alloc(u8, file_contents.len + 1) catch unreachable;
        defer gpa.free(file_sentinel);
        std.mem.copyForwards(u8, file_sentinel, file_contents);

        file_sentinel[file_contents.len] = 0;

        const model = model3d.load(file_sentinel[0..file_contents.len:0], readFn, null, null) orelse unreachable;
    
        const vertices: []model3d.Vertex = model.handle.vertex[0..model.handle.numvertex];

        const faces: []model3d.Face = model.handle.face[0..model.handle.numface];
        var buffer_toreturn = std.ArrayList(f32).init(gpa);
        for (faces) |face| {
            for (0..3) |i| {
                try buffer_toreturn.append(vertices[face.vertex[i]].x);
                try buffer_toreturn.append(vertices[face.vertex[i]].y);
                try buffer_toreturn.append(vertices[face.vertex[i]].z);
                try buffer_toreturn.append(vertices[face.normal[i]].x);
                try buffer_toreturn.append(vertices[face.normal[i]].y);
                try buffer_toreturn.append(vertices[face.normal[i]].z);
                if (textured) {    
                    try buffer_toreturn.append(model.handle.tmap[face.texcoord[i]].u);
                    try buffer_toreturn.append(1.0 - model.handle.tmap[face.texcoord[i]].v);
                }
            }
        }
        
        
        var texture_toreturn = std.ArrayList(u8).init(gpa);
        if (textured) {    
            var texture_data = model.textures()[0];
            std.debug.print("Texture size: {?}x{?}x{?}\n", .{texture_data.w, texture_data.h, texture_data.f});
            for (0..(@intCast(u32, texture_data.w) * @intCast(u32, texture_data.h))) |i| {
                try texture_toreturn.append(texture_data.d[i * 3]);
                try texture_toreturn.append(texture_data.d[i * 3 + 1]);
                try texture_toreturn.append(texture_data.d[i * 3 + 2]);
                try texture_toreturn.append(255);            
            }
        }
        return .{
            .buffer = buffer_toreturn.toOwnedSlice() catch unreachable,
            .texture = if (textured) (texture_toreturn.toOwnedSlice() catch unreachable) else null,
            .texture_width = if (textured) (model.textures()[0].w) else 0,
            .texture_height = if (textured) (model.textures()[0].h) else 0
        };
    }
};
