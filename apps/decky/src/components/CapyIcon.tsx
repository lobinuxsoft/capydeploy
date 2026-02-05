/**
 * Custom CapyDeploy icon for the sidebar.
 */

import { VFC } from "react";

interface CapyIconProps {
  size?: number;
  color?: string;
}

const CapyIcon: VFC<CapyIconProps> = ({ size = 24, color = "currentColor" }) => {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill={color}
      xmlns="http://www.w3.org/2000/svg"
    >
      {/* Simple capybara silhouette */}
      <path d="M12 2C8.5 2 5.5 4 4 7C2 7.5 1 9 1 11C1 13 2 14.5 4 15L4 18C4 20 6 22 9 22L15 22C18 22 20 20 20 18L20 15C22 14.5 23 13 23 11C23 9 22 7.5 20 7C18.5 4 15.5 2 12 2ZM8 10C8.8 10 9.5 10.7 9.5 11.5C9.5 12.3 8.8 13 8 13C7.2 13 6.5 12.3 6.5 11.5C6.5 10.7 7.2 10 8 10ZM16 10C16.8 10 17.5 10.7 17.5 11.5C17.5 12.3 16.8 13 16 13C15.2 13 14.5 12.3 14.5 11.5C14.5 10.7 15.2 10 16 10ZM10 15L14 15C14 16.1 13.1 17 12 17C10.9 17 10 16.1 10 15Z" />
    </svg>
  );
};

export default CapyIcon;
