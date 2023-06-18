--prints a csv from github repo transitland-atlas/feeds with data "feed,vehicles_url,trips_url,alerts_url,isauth,isswiftly"
local json = require("json")

function loadJsonFile(filePath)
    local file = io.open(filePath, "r")
    local contents = ""
    local myTable = {}
    if file ~= nil then
        local contents = file:read( "*a" )
        myTable = json.decode(contents);
        io.close(file)
        return myTable
    end
end

local directoryPath = "/path/to/transitland-atlas/feeds"

print("feed,vehicles_url,trips_url,alerts_url,isauth,isswiftly")
local file_list = io.popen("ls -1 "..directoryPath.."/*.json")
local output = file_list:read("*all")
file_list:close()
local json_files = {}
for line in output:gmatch("[^\r\n]+") do
    table.insert(json_files, line)
end

for i=1, #json_files do
    local jsonData = loadJsonFile(json_files[i])
    for i=1, #jsonData.feeds do
        local output_str = ""
        local data_feed = jsonData.feeds[i]
        if data_feed.spec == "gtfs-rt" and data_feed.urls ~= nil then
            output_str = tostring(data_feed.id)
            for i=0, #data_feed.urls do
                if data_feed.urls.realtime_vehicle_positions ~= nil then
                    output_str = output_str..","..data_feed.urls.realtime_vehicle_positions
                else
                    output_str = output_str..","
                end
                if data_feed.urls.realtime_trip_updates ~= nil then
                    output_str = output_str..","..data_feed.urls.realtime_trip_updates
                else
                    output_str = output_str..","
                end
                if data_feed.urls.realtime_alerts ~= nil then
                    output_str = output_str..","..data_feed.urls.realtime_alerts
                else
                    output_str = output_str..","
                end
            end
            if data_feed.authorization ~= nil then
                output_str = output_str..",true"
            else
                output_str = output_str..",false"
            end
            if data_feed.license ~= nil and data_feed.license.url == "https://www.goswift.ly/api-license" then
                output_str = output_str..",true"
            else
                output_str = output_str..",false"
            end
        end
        if output_str ~= "" then
            print(output_str)
        end
    end
end
