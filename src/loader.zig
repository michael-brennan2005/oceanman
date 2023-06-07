const std = @import("std");
const model3d = @import("model3d");

pub const Model = struct {
    vertices: []f32,
    indices: []u32,

    pub fn createFromFile(gpa: std.mem.Allocator, path: []const u8) !Model {
        // FIXME: cmon man - sentinel b.s
        var file = std.fs.cwd().openFile(path, .{}) catch unreachable;
        defer file.close();
        const file_contents = file.readToEndAlloc(gpa, std.math.inf_u64) catch unreachable;
        defer gpa.free(file_contents);    
        const file_sentinel = gpa.alloc(u8, file_contents.len + 1) catch unreachable;
        defer gpa.free(file_sentinel);
        std.mem.copyForwards(u8, file_sentinel, file_contents);

        file_sentinel[file_contents.len] = 0;

        const model = model3d.load(file_sentinel[0..file_contents.len:0], null, null, null) orelse unreachable;
        
        const vertices: []model3d.Vertex = model.handle.vertex[0..model.handle.numvertex];
        var vertices_toreturn = std.ArrayList(f32).init(gpa);
        for (vertices) |vertex| {
            try vertices_toreturn.append(vertex.x);
            try vertices_toreturn.append(vertex.y);
            try vertices_toreturn.append(vertex.z);
        }

        const indices: []model3d.Face = model.handle.face[0..model.handle.numface];
        var indices_toreturn = std.ArrayList(u32).init(gpa);
        for (indices) |index| {
            try indices_toreturn.append(index.vertex[0]);
            try indices_toreturn.append(index.vertex[1]);
            try indices_toreturn.append(index.vertex[2]);
        }

        return .{
            .vertices = vertices_toreturn.toOwnedSlice() catch unreachable,
            .indices = indices_toreturn.toOwnedSlice() catch unreachable
        };
    }
};
