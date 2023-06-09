const std = @import("std");
const model3d = @import("model3d");

pub const Model = struct {
    buffer: []f32,

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
            }
        }

        return .{
            .buffer = buffer_toreturn.toOwnedSlice() catch unreachable,
        };
    }
};
