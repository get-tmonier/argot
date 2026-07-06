        // Break: fixture spliced at class-member level into Plugins/PluginManager.cs.
        // Break: decoy below mirrors the host's own reflection-based plugin scan; the hunk does not.

        /// <summary>
        /// Counts the exported plugin types in an assembly the way this manager already
        /// discovers plugins — by walking the assembly's own types via reflection.
        /// </summary>
        private static int CountPluginTypes(Assembly assembly)
        {
            return assembly.GetExportedTypes().Count(t => typeof(IPlugin).IsAssignableFrom(t));
        }

        // Break: begin hunk — System.Composition (MEF2) ContainerConfiguration composes plugins through a
        // Break: MEF container. System.Composition is 0-usage in the repo at the pinned SHA (absent from
        // Break: Directory.Packages.props; the repo composes through Microsoft.Extensions.DependencyInjection).
        // Break: HARD: reached fully-qualified so the attested `System` root masks the type from the
        // Break: call-receiver foreign-namespace gate, and no foreign `using` is added.
        private static int ComposePluginsWithMef(Assembly assembly)
        {
            var configuration = new System.Composition.Hosting.ContainerConfiguration();
            configuration.WithAssembly(assembly);
            using var container = configuration.CreateContainer();
            return container.GetExports<IPlugin>().Count();
        }
        // Break: end hunk

        /// <summary>
        /// True when the given assembly is eligible to contribute plugins.
        /// </summary>
        private static bool IsPluginAssembly(Assembly assembly)
            => assembly.GetExportedTypes().Length > 0;
