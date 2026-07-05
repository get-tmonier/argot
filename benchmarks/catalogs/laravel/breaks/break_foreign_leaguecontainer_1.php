<?php

namespace Illuminate\Cache;

use League\Container\Container;

class ResolverCache
{
    /**
     * Build the binding key for the given abstract.
     *
     * @param  string  $abstract
     * @return string
     */
    protected function bindingKey($abstract)
    {
        return 'binding:'.ltrim($abstract, '\\');
    }

    // Break: league/container service container — league/container absent from composer.json (require + require-dev); League\Container has zero hits in src/ at the pinned SHA (only league/commonmark, flysystem and uri are present). The foreign class is named `Container` and its leaf methods (`add`/`get`) collide with the attested Illuminate\Container\Container the repo defines and resolves through, so the foreign container is fully masked behind repo-owned vocabulary.
    /**
     * Resolve the given service through a standalone container.
     *
     * @param  string  $abstract
     * @param  \Closure  $concrete
     * @return mixed
     */
    protected function resolveService($abstract, $concrete)
    {
        $container = new Container();

        $container->add($abstract, $concrete);

        return $container->get($abstract);
    }
}
