import { useShuriken } from "@/hooks/use-shuriken"

export default function Tools() {
    const { allShurikens } = useShuriken()
    const configurableShurikens = allShurikens.filter(
        s => s.tools && Object.keys(s.tools).length > 0
    )

    if (configurableShurikens.length === 0) {
        return (
        <div className="text-center text-muted-foreground mt-8">
            No Shurikens have configurable fields.
        </div>
        );
    }

    return (
        <div className="space-y-4">
            {configurableShurikens.map(shuriken => (
                <div key={shuriken.metadata.name} className="border rounded-md p-4 space-y-2">
                    <div className="flex justify-between items-center">
                        <h3 className="text-base font-semibold">{shuriken.metadata.name} Tools</h3>
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                        {Object.entries(shuriken.tools!).map(([toolName, tool]) => (
                            <div key={toolName} className="border p-3 rounded-md hover:shadow-lg transition-shadow duration-200">
                                <h4 className="font-medium">{tool.name}</h4>
                                <p className="text-sm text-muted-foreground">{tool.path}</p>
                            </div>
                        ))}
                    </div>
                </div>
            ))}
        </div>  
    );
}