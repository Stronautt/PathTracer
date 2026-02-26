/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   scene_4.c                                          :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: stronautt                                  +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2023/08/07 17:18:52 by stronautt         #+#    #+#             */
/*   Updated: 2023/08/07 17:18:37 by stronautt        ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "rt.h"

static void fig_func(char *name, char **payload, t_obj *p, t_scene *scene)       
{
    int			        i;
    static const char	*sys[] = {
        "\"type\"", "\"center\"", "\"center2\"", "\"normal\"",
	    "\"emission\"", "\"radius\"", "\"angle\"", "\"color\"",
        "\"material\"", "\"radius2\"", "\"specular\"", "\"texture\"",
        "\"scale\"", "\"center3\""};
    const void	        *data[] = {
        &p->type, &p->pos, &p->dir, &p->dir, &p->emission,
		&p->rad, &p->rad, &p->color, &p->material, &p->rad2, &p->spec,
		&p->id_tex, &p->scale, &p->dir2};
	static find_func    func[] = {(find_func)parse_figure_type,
        (find_func)parse_cl_float3, (find_func)parse_cl_float3,
        (find_func)parse_cl_float3, (find_func)parse_float,
        (find_func)parse_float, (find_func)parse_float,
        (find_func)parse_cl_float3, (find_func)parse_material,
        (find_func)parse_float, (find_func)parse_float,
		(find_func)parse_texture, (find_func)parse_float,
        (find_func)parse_cl_float3};

    i = -1;
    while (++i < (int)(sizeof(sys) / sizeof(char *)))
        FIND_FUNC(name, sys[i], func[i], payload, data[i], scene);
    i >= (int)(sizeof(sys) / sizeof(char *)) ? ERR("Figure property") : 0;
}

void		parse_figure(char **string, t_obj *p, t_scene *scene)
{
	char		*name;

	SOB(string);
	while (**string && **string != '}'
		&& (name = ft_strsub(*string, 0, GCI(*string))))
	{
		*string = *string + GCI(*string) + 1;
        fig_func(name, string, p, scene);
		SC(string, **string);
		free(name);
	}
	SCB(string);
}
