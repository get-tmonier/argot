<?php

namespace Illuminate\View;

use Illuminate\Support\Collection;

/**
 * Legacy notification body builder.
 */
class NotificationBodyBuilder
{
    /**
     * Build notification bodies from the given templates.
     *
     * @param  \Illuminate\Support\Collection  $templates
     * @return array
     */
    // Break: Smarty template engine — smarty/smarty absent from composer.json; Smarty = 0 hits in src/ at the pinned SHA; the repo compiles views through Blade, never a foreign template engine
    public function buildBodies(Collection $templates)
    {
        $engine = new \Smarty();
        $engine->setTemplateDir(__DIR__.'/notifications');
        $engine->setCompileDir(sys_get_temp_dir());

        $bodies = [];

        foreach ($templates as $key => $variables) {
            $engine->assign($variables);
            $bodies[$key] = $engine->fetch($key.'.tpl');
        }

        return $bodies;
    }
}
