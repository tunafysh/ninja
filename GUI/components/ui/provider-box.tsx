import { ReactNode, useState } from "react";

export default function ProviderBox({
  checked,
  name,
  icon,
  className = "",
  onCheck,
}: {
  checked: boolean;
  name: string;
  icon: ReactNode;
  className?: string;
  onCheck: () => void; // simple callback
}) {
  const [enabled, setEnabled] = useState<boolean>(checked);

  const toggle = () => {
    setEnabled((prev) => !prev);
    onCheck(); // just notify parent
  };

  return (
    <div
      onClick={toggle}
      className={`w-24 aspect-square flex flex-col p-1 justify-evenly items-center cursor-pointer select-none bg-muted outline-foreground ${
        enabled ? "outline-2" : "text-gray-500"
      } rounded-md transition-all ` + className}
    >
      <div className={enabled? "grayscale-0": "grayscale-100"}>
      {icon}
      </div>
      <h3 className="text-sm font-medium text-center">{name}</h3>
    </div>
  );
}
