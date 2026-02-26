/* **************************************************************************** */
/*                                                                              */
/*                                                         :::      ::::::::    */
/*    mandelbulb.cl                                      :+:      :+:    :+:    */
/*                                                     +:+ +:+         +:+      */
/*    By: omiroshn <marvin@42.fr>                    +#+  +:+       +#+         */
/*                                                 +#+#+#+#+#+   +#+            */
/*    Created: 2018/05/24 16:18:51 by omiroshn          #+#    #+#              */
/*    Updated: 2018/05/24 16:18:52 by omiroshn         ###   ########.fr        */
/*                                                                              */
/* **************************************************************************** */

static inline int dblsgn(float x)
{
	if (x < -EPSILON)
		return (-1);
	return (x > EPSILON);
}

static bool inside(float3 pt, t_obj obj)
{
	float	t = atan2(pt.y, pt.x);
	float	s = t;
	float	sin_v[2] = {sin(t), sin(t / 2.0F)};
	float	cos_v[2] = {cos(t), cos(t / 2.0F)};

	if (sin_v[1])
		s = pt.z / sin_v[1];
	else if (cos_v[0] && cos_v[1])
			s = (pt.x / cos_v[0] - obj.rad) / cos_v[1];
	else if (sin_v[0] && cos_v[1])
		s = (pt.y / sin_v[0] - obj.rad) / cos_v[1];
	pt.x -= (obj.rad + s * cos_v[1]) * cos_v[0];
	pt.y -= (obj.rad + s * cos_v[1]) * sin_v[0];
	pt.z -= s * sin_v[1];

	if (dblsgn(dot(pt, pt)))
		return false;
	return (s >= -obj.rad2 && s <= obj.rad2);
}

float2 intersect_ray_mebius(float3 O, float3 D, t_obj obj)
{
	float3	OC = O;

	t_obj lox = obj;
	lox.rad = obj.rad + obj.rad2;
	float2 sphere = intersect_ray_sphere(O, D, lox);
	if (sphere.x == INFINITY && sphere.y == INFINITY)
		return (sphere);

	float a1 = OC.x * OC.x * OC.y + OC.y * OC.y * OC.y - 2.0F * OC.x * OC.x * OC.z -
		2.0F * OC.y * OC.y * OC.z + OC.y * OC.z * OC.z - 2.0F * OC.x * OC.z * obj.rad - OC.y * obj.rad * obj.rad;
	float a2 = D.y * OC.x * OC.x - 2.0F * D.z * OC.x * OC.x + 2.0F * D.x * OC.x * OC.y +
		3.0F * D.y * OC.y * OC.y - 2.0F * D.z * OC.y * OC.y - 4.0F * D.x * OC.x * OC.z -
		4.0F * D.y * OC.y * OC.z + 2.0F * D.z * OC.y * OC.z + D.y * OC.z * OC.z -
		2.0F * D.z * OC.x * obj.rad - 2.0F * D.x * OC.z * obj.rad - D.y * obj.rad * obj.rad;
	float a3 = 2.0F * D.x * D.y * OC.x - 4.0F * D.x * D.z * OC.x + D.x * D.x * OC.y +
		3.0F * D.y * D.y * OC.y - 4.0F * D.y * D.z * OC.y + D.z * D.z * OC.y -
		2.0F * D.x * D.x * OC.z -2.0F * D.y * D.y * OC.z + 2.0F * D.y * D.z * OC.z - 2.0F * D.x * D.z * obj.rad;
	float a4 = D.x * D.x * D.y + D.y * D.y * D.y - 2.0F * D.x * D.x * D.z - 2.0F * D.y * D.y * D.z + D.y * D.z * D.z;

	float	roots[3];
	int		n_real_roots = 0;
	a3 /= a4, a2 /= a4, a1 /= a4;
	n_real_roots = SolveP3(roots, a3, a2, a1);

	float	min = INFINITY;
	for (int i = 0; i < n_real_roots; i++)
		if (roots[i] > 1.0F && roots[i] < min && inside(O + D * roots[i], obj))
			min = roots[i];
	return ((float2){min, INFINITY});
}
