const std = @import("std");

pub const c = @cImport({
    @cDefine("TINYOBJ_LOADER_C_IMPLEMENTATION", .{});
    @cInclude("tinyobj_loader_c.h");
});

pub const Object = struct {
    pub const Vertex = packed struct {
        x: f32,
        y: f32,
        z: f32
    };

    const log = std.log.scoped(.obj);

    vertices: std.ArrayList(Vertex),
    indices: std.ArrayList(u32),
    
    pub fn createFromFile(gpa: std.mem.Allocator, path: []const u8) Object {
        var file = std.fs.cwd().openFile(path, .{ .mode = std.fs.File.OpenMode.read_only }) catch unreachable;
        defer file.close();
        const file_contents = file.readToEndAlloc(gpa, std.math.inf_u64) catch unreachable;

        return createFromString(gpa, file_contents);
    }

    pub fn createFromString(gpa: std.mem.Allocator, string: []const u8) Object {
        var obj = Object {
            .vertices = std.ArrayList(Vertex).init(gpa),
            .indices = std.ArrayList(u32).init(gpa)
        };

        var line_number: usize = 1;
        var iterator = std.mem.split(u8, string, "\n");
        while (iterator.next()) |line| {
            var elements = std.mem.split(u8, line, " ");
            var typ = elements.next() orelse "";

            if (std.mem.eql(u8, typ, "v")) {
                log.debug("Parsing vertex: {s}", .{line});
                var x = elements.next();
                var y = elements.next();
                var z = elements.next();
                if (x == null or y == null or z == null) {
                    log.err("line {d}: {s} needs 3 vertices", .{line_number, line});
                    std.process.exit(1);                
                }

                obj.vertices.append(Vertex {
                    .x = std.fmt.parseFloat(f32, x.?) catch unreachable,
                    .y = std.fmt.parseFloat(f32, y.?) catch unreachable,
                    .z = std.fmt.parseFloat(f32, std.mem.trim(u8, z.?, " \n\r\t"))  catch unreachable
                }) catch unreachable;
            } else if (std.mem.eql(u8, typ, "f")) {
                log.debug("Parsing face: {s}", .{line});
                while (elements.next()) |vertex| {    
                    var indices = std.mem.split(u8, vertex, "/");

                    var i: usize = 0;
                    while (indices.next()) |index| {
                        if (i == 0) {
                            var index_num = std.fmt.parseInt(i32, index, 10) catch unreachable;

                            
                            var to_add = @intCast(u32, if (index_num < 0)
                                @intCast(i32, obj.vertices.items.len) + index_num else index_num - 1);
                            obj.indices.append(to_add) catch unreachable;
                        } else {
                            break;
                        }
                        i += 1;
                    }
                }
            }

            line_number += 1;
        }

        log.debug("Vertex count: {d}, Face count: {d}", .{obj.vertices.items.len, obj.indices.items.len / 3});
        log.debug("Vertex buffer: {any}", .{obj.vertices.items});
        log.debug("Index buffer: {any}", .{obj.indices.items});
        return obj;
    }
};

