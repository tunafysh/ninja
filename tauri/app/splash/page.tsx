import { Progress } from "@/components/ui/progress";

export default function Page() {
    return (
        <div className="flex flex-col items-center justify-center h-screen">
            <div>
                <p>Loading...</p>
                <br />
                <Progress
                    className="relative h-4 w-full overflow-hidden rounded-full bg-secondary"
                    
                />
    
            </div>
        </div>
    );
}