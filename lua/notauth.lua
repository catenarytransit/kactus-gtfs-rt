--writes to a log which gtfs-rt feeds do not need auth
local http = require("socket.http")
local csv = require("csv")

local function curl(url)
    local response = http.request(url)
    return response
end

local function logToFile(value)
    local file = io.open("log.txt", "a")
    file:write(value .. "\n")
    file:close()
end

local function processRecords(records)
    for record in records:lines() do
        local link = record[3]
        print(record[2])
        if link then
            local response = curl(link)
            if not response:find(">Authorization<") then
                local logText = record[2]
                logToFile(logText)
            end
        end
    end
end

local function processCSV(filePath)
    local records = csv.open(filePath)

    -- Create an array to store coroutine handlers
    local coroutines = {}

    for i = 1, 50 do
        -- Create a coroutine for each thread
        local co = coroutine.create(function()
            processRecords(records)
        end)
        table.insert(coroutines, co)
    end

    -- Resume each coroutine until they finish processing
    while #coroutines > 0 do
        for i = #coroutines, 1, -1 do
            local co = coroutines[i]
            local _, status = coroutine.resume(co)
            if not status or coroutine.status(co) == "dead" then
                table.remove(coroutines, i)
            end
        end
    end
end

processCSV("../agencies.csv")
