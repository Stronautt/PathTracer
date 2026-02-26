// #import types
// #import utils
// #import figures::sphere
// #import figures::plane
// #import figures::cube
// #import figures::cylinder
// #import figures::cone
// #import figures::torus
// #import figures::disc
// #import figures::triangle
// #import figures::skybox
// #import figures::mandelbulb
// #import figures::julia
// #import figures::ellipsoid
// #import figures::paraboloid
// #import figures::hyperboloid
// #import figures::mebius
// #import figures::pyramid
// #import figures::tetrahedron

// Dispatch intersection to the appropriate figure type.
fn intersect_figure(ray: Ray, idx: u32) -> HitRecord {
    let fig = figures[idx];
    var hit: HitRecord;

    switch fig.figure_type {
        case FIG_SPHERE: {
            hit = intersect_sphere(ray, fig);
        }
        case FIG_PLANE: {
            hit = intersect_plane(ray, fig);
        }
        case FIG_CUBE: {
            hit = intersect_cube(ray, fig);
        }
        case FIG_CYLINDER: {
            hit = intersect_cylinder(ray, fig);
        }
        case FIG_CONE: {
            hit = intersect_cone(ray, fig);
        }
        case FIG_TORUS: {
            hit = intersect_torus(ray, fig);
        }
        case FIG_DISC: {
            hit = intersect_disc(ray, fig);
        }
        case FIG_TRIANGLE: {
            hit = intersect_triangle(ray, fig);
        }
        case FIG_SKYBOX: {
            hit = intersect_skybox(ray, fig);
        }
        case FIG_MANDELBULB: {
            hit = intersect_mandelbulb(ray, fig);
        }
        case FIG_JULIA: {
            hit = intersect_julia(ray, fig);
        }
        case FIG_ELLIPSOID: {
            hit = intersect_ellipsoid(ray, fig);
        }
        case FIG_PARABOLOID: {
            hit = intersect_paraboloid(ray, fig);
        }
        case FIG_HYPERBOLOID: {
            hit = intersect_hyperboloid(ray, fig);
        }
        case FIG_MEBIUS: {
            hit = intersect_mebius(ray, fig);
        }
        case FIG_PYRAMID: {
            hit = intersect_pyramid(ray, fig);
        }
        case FIG_TETRAHEDRON: {
            hit = intersect_tetrahedron(ray, fig);
        }
        default: {
            hit.hit = false;
            hit.t = MAX_T;
        }
    }

    if hit.hit {
        hit.figure_idx = idx;
    }

    return hit;
}
