const std = @import("std");
const model3d = @import("model3d");


pub fn loadFromFile(gpa: std.mem.Allocator, path: []const u8) !void {
    var file = try std.fs.cwd().openFile(path, .{ .read = true });
    defer file.close();

    var contents = file.readToEndAlloc(gpa, std.math.inf_u64);

    // possible sentinel error incoming
    const model = model3d.load(contents, null, null, null) orelse unreachable;
    
    const vertices: []model3d.Vertex = model.handle.vertex[0..model.handle.numvertex];
    const vertices_toreturn = std.ArrayList(f32).init(gpa);
    for (vertices) |vertex| {
        try vertices_toreturn.append(vertex.x);
        try vertices_toreturn.append(vertex.y);
        try vertices_toreturn.append(vertex.z);
    }

    const indices: []model3d.Face = model.handle.face[0..model.handle.numface];
    _ = indices;
    
}