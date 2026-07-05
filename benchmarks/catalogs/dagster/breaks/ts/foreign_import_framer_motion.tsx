// Break: framer-motion <motion.div> / useAnimation instead of Dagit's styled-components + CSS transitions.
// Dagit styles and animates through @dagster-io/ui-components and styled-components (keyframes / transition
// props). framer-motion's motion components and useAnimation controls are a separate animation runtime absent
// from ui-core.
import * as React from 'react';
import {motion, useAnimation} from 'framer-motion';

export const AnimatedAssetBadge: React.FC<{live: boolean; label: string}> = ({live, label}) => {
  const controls = useAnimation();
  React.useEffect(() => {
    controls.start({
      scale: live ? [1, 1.15, 1] : 1,
      transition: {duration: 0.4, repeat: live ? Infinity : 0},
    });
  }, [live, controls]);
  return (
    <motion.div animate={controls} initial={{opacity: 0}} whileHover={{scale: 1.05}}>
      {label}
    </motion.div>
  );
};
